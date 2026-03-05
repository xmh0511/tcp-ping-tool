<script setup>
import { reactive, ref, onUnmounted, onMounted, computed } from "vue";
import { emit, listen } from "@tauri-apps/api/event";

import { SyncOutlined, CloseOutlined } from "@ant-design/icons-vue";
import { message } from "ant-design-vue";

function format(dat) {
  var year = dat.getFullYear();
  var mon =
    dat.getMonth() + 1 < 10 ? "0" + (dat.getMonth() + 1) : dat.getMonth() + 1;
  var data = dat.getDate() < 10 ? "0" + dat.getDate() : dat.getDate();
  var hour = dat.getHours() < 10 ? "0" + dat.getHours() : dat.getHours();
  var min = dat.getMinutes() < 10 ? "0" + dat.getMinutes() : dat.getMinutes();
  var seon = dat.getSeconds() < 10 ? "0" + dat.getSeconds() : dat.getSeconds();
  return year + "-" + mon + "-" + data + " " + hour + ":" + min + ":" + seon;
}

const per_pool = reactive({
  text: "",
  time_out_threshold: 5000,
  socks5_url: "",
  use_proxy: false,
  interval: 3000,
});

const record_vec = reactive({
  map: {},
});

const loading_state = ref(false);
const dialog_visible = ref(false);
const current_dialog_identity = ref(null);

let unlisten1;
let unlisten2;
let unlisten3;

onMounted(async () => {
  unlisten1 = await listen("reset", (event) => {
    message.error(`${event.payload}`);
    loading_state.value = false;
    dataSource.list = [];
    record_vec.map = {};
  });

  unlisten2 = await listen("complete", () => {
    message.success("本轮测试结束");
    loading_state.value = false;
  });

  unlisten3 = await listen("per-result", (event) => {
    let json = JSON.parse(event.payload);
    if (dataSource.list.length === 0) {
      return;
    }
    let index = json.index;
    if (dataSource.list[index] === undefined) {
      return;
    }
    if (json.success) {
      record_vec.map[json.ip].push({
        time: format(new Date()),
        value: json.msg.latency,
        is_error: false,
      });
      dataSource.list[index].latency.value = json.msg.latency;
      dataSource.list[index].latency.result = `${json.msg.latency} ms`;
      dataSource.list[index].latency.count = json.msg.count;
      dataSource.list[index].latency.average = `${json.msg.average} ms`;
      dataSource.list[index].latency.min = `${json.msg.min} ms`;
      dataSource.list[index].latency.packet_loss = json.msg.packet_loss;
      dataSource.list[index].latency.success = true;
      dataSource.list[index].is_pending = false;
    } else {
      record_vec.map[json.ip].push({
        time: format(new Date()),
        value: json.msg.error,
        is_error: true,
      });
      dataSource.list[index].latency.result = json.msg.error;
      dataSource.list[index].latency.count = json.msg.count;
      dataSource.list[index].latency.packet_loss = json.msg.packet_loss;
      dataSource.list[index].latency.success = false;
      dataSource.list[index].is_pending = false;
    }
  });
});

onUnmounted(() => {
  unlisten1();
  unlisten2();
  unlisten3();
});

const getLatencyClass = (record) => {
  if (record.is_pending) return "";
  if (!record.latency.success) return "red-text";
  const ms = record.latency.value;
  if (ms <= 100) return "green-text";
  if (ms <= 300) return "yellow-text";
  return "red-text";
};

const test_speed = async () => {
  if (loading_state.value === true) {
    emit("cancel-all");
    return;
  }
  let text = per_pool.text.trim();
  let text_n = text.replace(/\r/g, "");
  if (text_n === "") {
    message.error(`填写测试地址`);
    return;
  }
  let arr = text_n
    .split("\n")
    .map((s) => s.trim())
    .filter((s) => s !== "");
  if (arr.length === 0) {
    message.error(`填写测试地址`);
    return;
  }
  dataSource.list = [];
  record_vec.map = {};
  let data_index = 0;
  for (let per of arr) {
    record_vec.map[per] = [];
    dataSource.list.push({
      per,
      data_index,
      latency: {
        result: "",
        success: true,
        count: 0,
        average: "—",
        min: "—",
        packet_loss: 0,
        value: 0,
      },
      is_pending: true,
    });
    data_index++;
  }
  loading_state.value = true;
  emit("test-pers", {
    pers: arr,
    time_out: per_pool.time_out_threshold,
    use_proxy: per_pool.use_proxy,
    socks5_url: per_pool.socks5_url,
    interval: per_pool.interval,
  });
};

const cleanAll = () => {
  if (loading_state.value === false) {
    dataSource.list = [];
    record_vec.map = {};
  }
};

const dataSource = reactive({
  list: [],
});

const columns = reactive([
  {
    title: "远端",
    dataIndex: "per",
    key: "per",
  },
  {
    title: "延迟",
    dataIndex: "latency",
    key: "latency",
  },
  {
    title: "平均延迟",
    key: "average",
  },
  {
    title: "最小延迟",
    key: "min",
  },
  {
    title: "次数",
    dataIndex: "count",
    key: "count",
  },
  {
    title: "丢包率",
    key: "packet_loss",
  },
]);

const customRow = (record) => {
  return {
    onClick: () => {
      current_dialog_identity.value = record.per;
      dialog_visible.value = true;
    },
  };
};

const closeModal = () => {
  dialog_visible.value = false;
};

const reverse_list = computed(() => {
  if (current_dialog_identity.value === null) {
    return [];
  }
  if (record_vec.map[current_dialog_identity.value] === undefined) {
    return [];
  }
  return [...record_vec.map[current_dialog_identity.value]].reverse();
});
</script>

<template>
  <div class="card">
    <div class="modal-panel" v-if="dialog_visible">
      <div class="modal-inner-panel">
        <div class="modal-body">
          <div class="modal-body-bar">
            <div class="modal-title">
              <span>{{ current_dialog_identity }}</span>
            </div>
            <div class="modal-close-button">
              <CloseOutlined @click="closeModal" />
            </div>
          </div>
          <div class="modal-body-content">
            <p v-for="(item, key) in reverse_list" :key="key">
              <span>{{ item.time }}</span>
              <span style="margin-right: 10px">:</span>
              <span :class="item.is_error ? 'red-text' : ''">
                {{ item.is_error ? item.value : item.value + " ms" }}
              </span>
            </p>
          </div>
        </div>
      </div>
    </div>
    <div class="form-content">
      <a-form>
        <a-form-item label="超时阈值" :label-col="{ span: 3 }">
          <a-input
            v-model:value="per_pool.time_out_threshold"
            suffix="ms"
          ></a-input>
        </a-form-item>
        <a-form-item label="间隔时间" :label-col="{ span: 3 }">
          <a-input v-model:value="per_pool.interval" suffix="ms"></a-input>
        </a-form-item>
        <a-form-item label="Socks5" :label-col="{ span: 3 }">
          <div class="group-in-line">
            <a-input
              :disabled="!per_pool.use_proxy"
              v-model:value="per_pool.socks5_url"
            ></a-input>
            <a-checkbox
              class="checkbox-item"
              v-model:checked="per_pool.use_proxy"
            ></a-checkbox>
          </div>
        </a-form-item>
        <a-form-item label="Tcp测试池" name="name" :label-col="{ span: 3 }">
          <a-textarea
            class="per-pool-textarea"
            v-model:value="per_pool.text"
            placeholder="每个地址:端口号独占一行"
          />
        </a-form-item>
      </a-form>
      <div class="button-content">
        <a-button
          style="margin-right: 10px"
          type="primary"
          @click="test_speed"
          >{{ loading_state === false ? "开始" : "结束" }}</a-button
        >
        <a-button
          :disabled="loading_state"
          type="primary"
          danger
          @click="cleanAll"
          >清除</a-button
        >
      </div>
    </div>
    <div style="height: 20px; width: 100%">
      <a-badge
        v-if="loading_state === true"
        status="processing"
        text="进行中"
      />
    </div>
    <div class="table-content">
      <a-table
        :dataSource="dataSource.list"
        :columns="columns"
        :pagination="false"
        :customRow="customRow"
        class="clickable-table"
      >
        <template #bodyCell="{ column, record }">
          <template v-if="column.key === 'latency'">
            <p
              v-if="record.is_pending === false"
              class="latency-text"
              :class="getLatencyClass(record)"
            >
              {{ record.latency.result }}
            </p>
            <SyncOutlined v-else spin></SyncOutlined>
          </template>
          <template v-if="column.key === 'average'">
            <p class="latency-text" :class="getLatencyClass(record)">
              {{ record.latency.average }}
            </p>
          </template>
          <template v-if="column.key === 'min'">
            <p class="latency-text" :class="getLatencyClass(record)">
              {{ record.latency.min }}
            </p>
          </template>
          <template v-if="column.key === 'count'">
            <a-statistic :value="record.latency.count" />
          </template>
          <template v-if="column.key === 'packet_loss'">
            <template v-if="!record.is_pending">
              <p
                class="latency-text"
                :class="
                  record.latency.packet_loss > 0 ? 'red-text' : 'green-text'
                "
              >
                {{ record.latency.packet_loss }}%
              </p>
            </template>
            <template v-else>
              <span>—</span>
            </template>
          </template>
        </template>
      </a-table>
    </div>
  </div>
</template>

<style scoped>
.card {
  width: 80%;
  margin: 0 auto;
}
.per-pool-textarea {
  height: 150px;
  resize: none;
}
.table-content {
  margin-top: 10px;
  max-height: 260px;
  overflow-y: auto;
  padding-bottom: 20px;
}
.latency-text {
  margin-bottom: 0px;
}
.green-text {
  color: #52c41a;
}
.yellow-text {
  color: #faad14;
}
.red-text {
  color: #ff4d4f;
}
.form-content {
  padding-top: 15px;
}
.button-content {
  text-align: right;
}
.group-in-line {
  display: flex;
  align-items: center;
}
.group-in-line .checkbox-item {
  margin-left: 10px;
}
:deep(.ant-statistic-content .ant-statistic-content-value) {
  font-size: 16px !important;
}
:deep(.ant-statistic-content-value-int) {
  font-size: 16px !important;
}
.modal-panel {
  position: absolute;
  top: 0px;
  bottom: 0px;
  left: 0px;
  right: 0px;
  background-color: rgba(0, 0, 0, 0.5);
  z-index: 9999;
}
.modal-inner-panel {
  width: 80%;
  height: 60%;
  background-color: white;
  margin-left: auto;
  margin-right: auto;
  margin-top: 20%;
  border-radius: 10px;
}
.modal-body-bar {
  height: 39px;
  border-bottom: 1px solid #f1efef;
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.modal-close-button {
  margin-right: 8px;
  cursor: pointer;
}
.modal-title {
  font-weight: bold;
  margin-left: 10px;
}
.modal-body {
  display: flex;
  flex-direction: column;
  height: 100%;
  padding-bottom: 10px;
}
.modal-body-content {
  flex: 1;
  overflow: auto;
  padding: 10px;
  box-sizing: border-box;
}
.clickable-table :deep(tr.ant-table-row) {
  cursor: pointer;
}
.clickable-table :deep(tr.ant-table-row:hover > td) {
  background-color: #e6f7ff !important;
}
</style>
