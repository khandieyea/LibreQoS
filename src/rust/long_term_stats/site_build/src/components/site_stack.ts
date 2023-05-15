import { scaleNumber } from "../helpers";
import { Component } from "./component";
import * as echarts from 'echarts';

export class SiteStackChart implements Component {
    div: HTMLElement;
    myChart: echarts.ECharts;
    chartMade: boolean = false;
    siteId: string;

    constructor(siteId: string) {
        this.siteId = siteId;
        this.div = document.getElementById("siteStack") as HTMLElement;
        this.myChart = echarts.init(this.div);
        this.myChart.showLoading();
    }

    wireup(): void {
    }

    ontick(): void {
        window.bus.requestThroughputStackSite(this.siteId);
    }

    onmessage(event: any): void {
        if (event.msg == "siteStack") {
            let series: echarts.SeriesOption[] = [];

            // Iterate all provides nodes and create a set of series for each,
            // providing upload and download banding per node.
            let x: any[] = [];
            let first = true;
            let legend: string[] = [];
            for (let i = 0; i < event.nodes.length; i++) {
                let node = event.nodes[i];
                if (node.node_name != "Root") {
                    legend.push(node.node_name);
                    //legend.push(node.node_name + " UL");
                    //console.log(node);

                    let d: number[] = [];
                    let u: number[] = [];
                    let l: number[] = [];
                    for (let j = 0; j < node.down.length; j++) {
                        if (first) x.push(node.down[j].date);
                        d.push(node.down[j].value);
                        u.push(node.down[j].u);
                        l.push(node.down[j].l);
                    }
                    if (first) first = false;

                    let val: echarts.SeriesOption = {
                        name: node.node_name,
                        type: "line",
                        data: d,
                        symbol: 'none',
                        stack: 'download',
                        areaStyle: {},
                    };

                    series.push(val);

                    // Do the same for upload
                    d = [];
                    u = [];
                    l = [];
                    for (let j = 0; j < node.down.length; j++) {
                        d.push(0.0 - node.up[j].value);
                        u.push(0.0 - node.up[j].u);
                        l.push(0.0 - node.up[j].l);
                    }

                    val = {
                        name: node.node_name,
                        type: "line",
                        data: d,
                        symbol: 'none',
                        stack: 'upload',
                        areaStyle: {},
                        label: { show: false }
                    };

                    series.push(val);
                }
            }

            if (!this.chartMade) {
                this.myChart.hideLoading();
                var option: echarts.EChartsOption;
                this.myChart.setOption<echarts.EChartsOption>(
                    (option = {
                        title: { text: "Child Node Throughput (Bits)" },
                        legend: {
                            orient: "vertical",
                            right: 0,
                            top: "bottom",
                            data: legend,
                            textStyle: { fontSize: 8 }
                        },
                        xAxis: {
                            type: 'category',
                            data: x,
                            position: 'top',
                        },
                        yAxis: {
                            type: 'value',
                            axisLabel: {
                                formatter: function (val: number) {
                                    return scaleNumber(Math.abs(val));
                                }
                            }
                        },
                        series: series
                    })
                );
                option && this.myChart.setOption(option);
                // this.chartMade = true;
            }
        }
    }
}