<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

const imageUrl = ref("https://a.vpimg3.com/upload/merchandise/27678/ID-4535147741623-1.jpg");
const isImageLoading = ref(true);
const imageError = ref(false);

function handleImageLoad() {
  isImageLoading.value = false;
}

function handleImageError() {
  isImageLoading.value = false;
  imageError.value = true;
}

const ticket = ref("");
const serverPort = ref(8080);
const status = ref<any>({});
const isServerRunning = ref(false);
const isConnected = ref(false);
const testUrl = ref("http://iroh.localhost/v1/models");
const testResult = ref("");
const isLoading = ref(false);
 
async function connectToPeer() {
  if (!ticket.value) {
    alert("Please enter a ticket to connect");
    return;
  }
  try {
    await invoke("connect_to_peer", { ticket: ticket.value });
    isConnected.value = true;
    await updateStatus();
  } catch (e) {
    console.error("Failed to connect:", e);
    alert("Failed to connect: " + e);
  }
}

async function disconnect() {
  try {
    await invoke("disconnect_peer");
    isConnected.value = false;
    await updateStatus();
  } catch (e) {
    console.error("Failed to disconnect:", e);
  }
}

async function updateStatus() {
  try {
    const result = await invoke<string>("get_connection_status");
    status.value = JSON.parse(result);
    isServerRunning.value = status.value.server_active;
    isConnected.value = status.value.connected;
  } catch (e) {
    console.error("Failed to get status:", e);
  }
}

async function testHttpViaP2p() {
  if (!isConnected.value) {
    alert("Please connect to a peer first");
    return;
  }
  isLoading.value = true;
  testResult.value = "";
  try {
    const result = await invoke<any>("http_via_p2p", {
      method: "GET",
      url: testUrl.value,
      headers: [],
      body: null
    });
    testResult.value = JSON.stringify(result, null, 2);
  } catch (e) {
    console.error("Failed to make request:", e);
    testResult.value = "Error: " + e;
  } finally {
    isLoading.value = false;
  }
}

async function testDirectHttp() {
  console.log("test ", testUrl.value);
  try {
    const res = await fetch(testUrl.value);
    const json = await res.json();
    testResult.value = JSON.stringify(json, null, 2);
  } catch (err) {
    console.error("Failed to make direct request:", err);
    testResult.value = "Error: " + err;
  }
}

async function copyTicket() {
  if (ticket.value) {
    await navigator.clipboard.writeText(ticket.value);
    alert("Ticket copied to clipboard!");
  }
}

onMounted(() => {
  updateStatus();
});
</script>

<template>
  <main class="container">
    <h1>P2P Proxy via Dumbpipe</h1>

    
     

    <div class="card">
      <h2>Test HTTP Request</h2>
      <div class="form-group">
        <label>URL:</label>
        <input type="text" v-model="testUrl" />
      </div>
      <div class="button-group"> 
        <button @click="testDirectHttp" :disabled="isLoading" class="secondary">
          Direct Request
        </button>
      </div>
      <pre v-if="testResult" class="result">{{ testResult }}</pre>
    </div>

    <div class="card">
      <h2>Network Image</h2>
      <div class="form-group">
        <label>Image URL:</label>
        <input type="text" v-model="imageUrl" />
      </div>
      <div class="image-container">
        <div v-if="isImageLoading" class="loading">Loading image...</div>
        <div v-else-if="imageError" class="error">Failed to load image</div>
        <img 
          v-else 
          :src="imageUrl" 
          alt="Network Image" 
          @load="handleImageLoad"
          @error="handleImageError"
        />
      </div>
    </div>
  </main>
</template>

<style scoped>
.container {
  max-width: 800px;
  margin: 0 auto;
  padding: 20px;
}

.card {
  border: 1px solid #ddd;
  border-radius: 8px;
  padding: 20px;
  margin-bottom: 20px;
  background: #f9f9f9;
}

h2 {
  margin-top: 0;
}

.status-info span {
  font-weight: bold;
}

.status-info span.active {
  color: green;
}

.form-group {
  margin-bottom: 15px;
}

.form-group label {
  display: block;
  margin-bottom: 5px;
}

.form-group input {
  width: 100%;
  padding: 8px;
  border: 1px solid #ccc;
  border-radius: 4px;
}

.button-group {
  display: flex;
  gap: 10px;
  margin-bottom: 10px;
}

button {
  padding: 10px 20px;
  background: #007bff;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
}

button:hover:not(:disabled) {
  background: #0056b3;
}

button:disabled {
  background: #ccc;
  cursor: not-allowed;
}

button.secondary {
  background: #6c757d;
}

button.secondary:hover:not(:disabled) {
  background: #545b62;
}

button.small {
  padding: 5px 10px;
  font-size: 12px;
}

code {
  background: #e9ecef;
  padding: 2px 6px;
  border-radius: 3px;
  word-break: break-all;
}

.result {
  background: #272822;
  color: #f8f8f2;
  padding: 15px;
  border-radius: 4px;
  overflow-x: auto;
  max-height: 400px;
}

.image-container {
  min-height: 200px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #f0f0f0;
  border-radius: 4px;
  overflow: hidden;
}

.image-container img {
  max-width: 100%;
  max-height: 400px;
  object-fit: contain;
}

.image-container .loading,
.image-container .error {
  padding: 20px;
  color: #666;
}

.image-container .error {
  color: #dc3545;
}
</style>
