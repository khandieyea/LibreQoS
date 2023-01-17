use anyhow::Result;
use serde::Serialize;
use crate::queue_types::QueueType;

#[derive(Debug, Clone, Serialize)]
pub enum QueueDiff {
    None,
    //    Mq,
    //    Htb,
    //    FqCodel,
    Cake(CakeDiff),
    //    ClsAct,
}

pub(crate) fn make_queue_diff(previous: &QueueType, current: &QueueType) -> Result<QueueDiff> {
    match previous {
        QueueType::Cake(..) => match current {
            QueueType::Cake(..) => Ok(cake_diff(previous, current)?),
            _ => Err(anyhow::Error::msg("Not implemented")),
        },
        _ => Err(anyhow::Error::msg("Not implemented")),
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct CakeDiff {
    pub bytes: u64,
    pub packets: u32,
    pub qlen: u32,
    pub tins: Vec<CakeDiffTin>,
}

#[derive(Serialize, Clone, Debug)]
pub struct CakeDiffTin {
    pub sent_bytes: u64,
    pub backlog_bytes: u32,
    pub drops: u32,
    pub marks: u32,
    pub avg_delay_us: u32,
}

fn cake_diff(previous: &QueueType, current: &QueueType) -> Result<QueueDiff> {
    // TODO: Wrapping Handler
    if let QueueType::Cake(prev) = previous {
        if let QueueType::Cake(new) = current {
            let tins = new
                .tins
                .iter()
                .zip(prev.tins.iter())
                .map(|(new, prev)| {
                    CakeDiffTin {
                        sent_bytes: new.sent_bytes.checked_sub(prev.sent_bytes).unwrap_or(0),
                        backlog_bytes: new.backlog_bytes,
                        drops: new.drops - prev.drops,
                        marks: new.ecn_marks.checked_sub(prev.ecn_marks).unwrap_or(0),
                        avg_delay_us: new.avg_delay_us,
                    }
                })
                .collect();
            return Ok(QueueDiff::Cake(CakeDiff {
                bytes: new.bytes - prev.bytes,
                packets: new.packets - prev.packets,
                qlen: new.qlen,
                tins,
            }));
        }
    }
    Err(anyhow::Error::msg("Not implemented"))
}