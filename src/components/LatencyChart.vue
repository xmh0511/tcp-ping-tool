<script setup>
import { ref, watch, onMounted, onUnmounted, nextTick } from "vue";
import * as echarts from "echarts";

const props = defineProps({
  records: {
    type: Array,
    default: () => [],
  },
  title: {
    type: String,
    default: "",
  },
});

const chartContainer = ref(null);
let chart = null;

function latencyColor(ms) {
  if (ms <= 100) return "#52c41a";
  if (ms <= 300) return "#faad14";
  return "#ff4d4f";
}

function buildOptions(records) {
  const xLabels = records.map((_, i) => i + 1);

  // Success series: line connecting non-error points (nulls break the line)
  const lineData = records.map((r) => {
    if (r.is_error) return { value: null };
    return {
      value: r.value,
      itemStyle: { color: latencyColor(r.value) },
    };
  });

  // Error series: scatter markers at y=0 to show where failures occurred
  const errorData = records
    .map((r, i) => (r.is_error ? { value: [i, 0], name: r.value } : null))
    .filter(Boolean);

  return {
    grid: { left: 46, right: 12, top: 20, bottom: 28 },
    tooltip: {
      trigger: "axis",
      axisPointer: { type: "cross", snap: true },
      formatter(params) {
        const idx = params[0].dataIndex;
        const r = records[idx];
        if (!r) return "";
        if (r.is_error) {
          return `<b>#${idx + 1}</b> ${r.time}<br/><span style="color:#ff4d4f">✕ ${r.value}</span>`;
        }
        return `<b>#${idx + 1}</b> ${r.time}<br/>延迟: <span style="color:${latencyColor(r.value)}"><b>${r.value} ms</b></span>`;
      },
    },
    xAxis: {
      type: "category",
      data: xLabels,
      name: "次",
      nameGap: 4,
      nameTextStyle: { fontSize: 11 },
      axisLabel: { fontSize: 11 },
    },
    yAxis: {
      type: "value",
      name: "ms",
      nameTextStyle: { fontSize: 11 },
      axisLabel: { fontSize: 11 },
      min: 0,
    },
    series: [
      {
        name: "延迟",
        type: "line",
        data: lineData,
        connectNulls: false,
        smooth: false,
        symbol: "circle",
        symbolSize: 5,
        lineStyle: { width: 2, color: "#1890ff" },
        itemStyle: { color: "#1890ff" },
      },
      {
        name: "失败",
        type: "scatter",
        data: errorData.map((d) => d.value),
        symbol: "path://M-1,-1L1,1M1,-1L-1,1",
        symbolSize: 10,
        itemStyle: { color: "#ff4d4f" },
        tooltip: {
          formatter(params) {
            const origItem = errorData[params.dataIndex];
            const r = records[origItem.value[0]];
            return `<b>#${origItem.value[0] + 1}</b> ${r.time}<br/><span style="color:#ff4d4f">✕ ${r.value}</span>`;
          },
        },
      },
    ],
  };
}

function initChart() {
  if (chartContainer.value && !chart) {
    chart = echarts.init(chartContainer.value);
    chart.setOption(buildOptions(props.records));
  }
}

watch(
  () => props.records,
  (newRecords) => {
    if (chart) {
      chart.setOption(buildOptions(newRecords), { notMerge: true });
    }
  },
  { deep: true }
);

onMounted(() => {
  nextTick(initChart);
});

onUnmounted(() => {
  if (chart) {
    chart.dispose();
    chart = null;
  }
});
</script>

<template>
  <div ref="chartContainer" class="chart-container"></div>
</template>

<style scoped>
.chart-container {
  width: 100%;
  height: 240px;
}
</style>
