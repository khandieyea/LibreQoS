use crate::web::wss::{queries::time_period::InfluxTimePeriod, send_response};
use axum::extract::ws::WebSocket;
use pgdb::{sqlx::{Pool, Postgres}, organization_cache::get_org_details};
use tracing::{instrument, error};
use wasm_pipe_types::SiteStackHost;

#[derive(Debug, influxdb2::FromDataPoint)]
pub struct SiteStackRow {
    pub node_name: String,
    pub node_parents: String,
    pub bits_max: i64,
    pub time: chrono::DateTime<chrono::FixedOffset>,
    pub direction: String,
}

impl Default for SiteStackRow {
    fn default() -> Self {
        Self {
            node_name: "".to_string(),
            node_parents: "".to_string(),
            bits_max: 0,
            time: chrono::DateTime::<chrono::Utc>::MIN_UTC.into(),
            direction: "".to_string(),
        }
    }
}

#[instrument(skip(cnn, socket, key, period))]
pub async fn send_site_stack_map(
    cnn: &Pool<Postgres>,
    socket: &mut WebSocket,
    key: &str,
    period: InfluxTimePeriod,
    site_id: String,
) -> anyhow::Result<()> {
    let site_index = pgdb::get_site_id_from_name(cnn, key, &site_id).await?;

    if let Some(org) = get_org_details(cnn, key).await {
        let influx_url = format!("http://{}:8086", org.influx_host);
        let client = influxdb2::Client::new(influx_url, &org.influx_org, &org.influx_token);
        let qs = format!("import \"strings\"

        from(bucket: \"{}\")
        |> {}
        |> filter(fn: (r) => r[\"_field\"] == \"bits_max\")
        |> filter(fn: (r) => r[\"_measurement\"] == \"tree\")
        |> filter(fn: (r) => r[\"organization_id\"] == \"{}\")
        |> filter(fn: (r) => strings.hasSuffix(v: r[\"node_parents\"], suffix: \"S{}S\" + r[\"node_index\"] + \"S\" ))
        |> group(columns: [\"node_name\", \"node_parents\", \"_field\", \"node_index\", \"direction\"])
        |> {}
        |> yield(name: \"last\")",
        org.influx_bucket, period.range(), org.key, site_index, period.aggregate_window_sum());

        //println!("{qs}");

        let query = influxdb2::models::Query::new(qs);
        //let rows = client.query_raw(Some(query)).await;
        let rows = client.query::<SiteStackRow>(Some(query)).await;
        match rows {
            Err(e) => error!("Influxdb tree query error: {e}"),
            Ok(rows) => {
                let mut result: Vec<SiteStackHost> = Vec::new();
                for row in rows.iter() {
                    if let Some(r) = result.iter_mut().find(|r| r.node_name == row.node_name) {
                        if row.direction == "down" {
                            r.download.push((
                                row.time.format("%Y-%m-%d %H:%M:%S").to_string(),
                                row.bits_max
                            ));
                        } else {
                            r.upload.push((
                                row.time.format("%Y-%m-%d %H:%M:%S").to_string(),
                                row.bits_max
                            ));
                        }
                    } else if row.direction == "down" {
                        result.push(SiteStackHost { 
                            node_name: row.node_name.clone(),
                            download: vec![(row.time.format("%Y-%m-%d %H:%M:%S").to_string(), row.bits_max)],
                            upload: vec![],
                        });
                    } else {
                        result.push(SiteStackHost { 
                            node_name: row.node_name.clone(),
                            upload: vec![(row.time.format("%Y-%m-%d %H:%M:%S").to_string(), row.bits_max)],
                            download: vec![],
                        });
                    }
                }

                // Sort descending by total
                result.sort_by(|a, b| {
                    b.total()
                        .partial_cmp(&a.total())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                const MAX_HOSTS: usize = 8;
                if result.len() > MAX_HOSTS {
                    let mut others = SiteStackHost {
                        node_name: "others".to_string(),
                        download: Vec::new(),
                        upload: Vec::new(),
                    };
                    result[0].download.iter().for_each(|x| {
                        others.download.push((x.0.clone(), 0));
                        others.upload.push((x.0.clone(), 0));
                    });
                    result.iter().skip(MAX_HOSTS).for_each(|row| {
                        row.download.iter().enumerate().for_each(|(i, x)| {
                            if i < others.download.len() {
                                others.download[i].1 += x.1;
                            }
                        });
                        row.upload.iter().enumerate().for_each(|(i, x)| {
                            if i < others.upload.len() {
                                others.upload[i].1 += x.1;
                            }
                        });
                    });
                    result.truncate(MAX_HOSTS);
                    result.push(others);
                }

                // Send the reply
                send_response(
                    socket,
                    wasm_pipe_types::WasmResponse::SiteStack { nodes: result },
                )
                .await;
            }
        }
    }

    Ok(())
}
