<template>
  <ModalDialog :title="title" :width="620" @cancel="close">
    <div class="flex flex-col gap-3 p-3 pt-0 text-sm">
      <!-- what this photo has become so far; click one to build on it -->
      <div v-if="versions.length" class="flex flex-col gap-1 max-h-40 overflow-y-auto">
        <button
          v-for="v in versions"
          :key="v.id"
          class="flex items-center gap-2 px-2 py-1 rounded text-left hover:bg-base-content/10"
          :class="{ 'bg-base-content/10': basedOn === v.id }"
          :title="v.path"
          @click="basedOn = basedOn === v.id ? null : v.id"
        >
          <span class="opacity-50 w-8 shrink-0">v{{ v.id }}</span>
          <span class="truncate flex-1">{{ v.prompt }}</span>
          <span class="opacity-50 shrink-0">{{ v.method === 'generated' ? 'generated' : 'adjusted' }}</span>
          <span v-if="!v.exists" class="opacity-50 shrink-0">file gone</span>
        </button>
      </div>
      <p v-else class="opacity-60">No versions yet. The original is never changed.</p>

      <!-- one tap for the usual things -->
      <div class="flex flex-wrap gap-2">
        <button
          v-for="p in presets"
          :key="p.label"
          class="px-2 py-1 rounded border border-base-content/20 hover:bg-base-content/10"
          @click="prompt = p.prompt"
        >{{ p.label }}</button>
      </div>

      <textarea
        ref="input"
        v-model="prompt"
        rows="3"
        class="w-full rounded bg-base-100/60 border border-base-content/20 p-2 outline-none"
        :placeholder="placeholder"
        @keydown.ctrl.enter.prevent="send"
        @keydown.meta.enter.prevent="send"
      ></textarea>

      <p v-if="error" class="text-error">{{ error }}</p>
      <p v-else-if="queued" class="opacity-70">
        Queued. It keeps going if you close this: an adjustment takes about 20 seconds, a generated edit a minute or two.
      </p>

      <div class="flex items-center gap-2">
        <span class="opacity-60 flex-1 truncate">
          {{ basedOn ? `Building on version ${basedOn}` : 'Building on the original' }} · results in ~/Pictures/Photos Edits
        </span>
        <button class="px-3 py-1 rounded border border-base-content/20 hover:bg-base-content/10" @click="close">Close</button>
        <button
          class="px-3 py-1 rounded bg-primary text-primary-content disabled:opacity-40"
          :disabled="!prompt.trim() || sending"
          @click="send"
        >{{ sending ? 'Sending…' : 'Ask' }}</button>
      </div>

      <!-- what is running right now, anywhere in the library -->
      <div v-if="running.length" class="flex flex-col gap-1 border-t border-base-content/10 pt-2">
        <div v-for="j in running" :key="j.id" class="flex items-center gap-2 opacity-70">
          <span class="w-8 shrink-0">{{ j.state === 'running' ? '···' : '·' }}</span>
          <span class="truncate flex-1">{{ j.prompt }}</span>
          <span class="shrink-0">{{ j.state }}</span>
        </div>
      </div>
    </div>
  </ModalDialog>
</template>

<script setup lang="ts">
/**
 * Ask about this photo: a conversation that produces versions.
 *
 * Every request goes to the tool that maintains the album, which queues it and
 * writes the result as a new file. The original is never touched. Clicking a
 * version makes the next request build on it, which is how a conversation
 * ("warmer" then "now black and white") stays coherent.
 */
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import ModalDialog from '@/components/ModalDialog.vue';

const props = defineProps<{ fileId: number; fileName?: string }>();
const emit = defineEmits<{ (e: 'close'): void; (e: 'versions', count: number): void }>();

type Version = { id: number; prompt: string; method?: string; path: string; exists: boolean; parent?: number | null };
type Job = { id: number; state: string; prompt: string; variant?: number | null; error?: string | null };

const versions = ref<Version[]>([]);
const jobs = ref<Job[]>([]);
const prompt = ref('');
const basedOn = ref<number | null>(null);
const sending = ref(false);
const queued = ref(false);
const error = ref('');
const input = ref<HTMLTextAreaElement | null>(null);
let timer: number | undefined;

const presets = [
  { label: 'Restore', prompt: 'Restore this old photo: repair damage, remove scratches and noise, and make it sharp, keeping the same people and framing.' },
  { label: 'Enhance', prompt: 'Improve exposure, contrast and colour, naturally, without changing the content.' },
  { label: 'Black and white', prompt: 'Convert to black and white with good contrast.' },
  { label: 'Straighten', prompt: 'Straighten the horizon and crop to the largest sensible rectangle.' },
  { label: 'Remove background', prompt: 'Remove the background, keeping the main subject, on a plain white background.' },
];

const title = computed(() => (props.fileName ? `Ask about ${props.fileName}` : 'Ask about this photo'));
const placeholder = computed(() =>
  basedOn.value
    ? `Change version ${basedOn.value}…`
    : 'What should change? For example: remove the plate, restore this old photo, warmer light',
);
const running = computed(() => jobs.value.filter((j) => j.state === 'queued' || j.state === 'running'));

async function refresh() {
  try {
    const v = (await invoke('photo_versions', { fileId: props.fileId })) as { versions: Version[] };
    const before = versions.value.length;
    versions.value = v.versions ?? [];
    if (versions.value.length !== before) emit('versions', versions.value.length);
  } catch (e) {
    // a photo whose album has no tool simply has no versions
  }
  try {
    const j = (await invoke('photo_edit_jobs', { limit: 8 })) as { jobs: Job[] };
    jobs.value = j.jobs ?? [];
  } catch (e) {
    jobs.value = [];
  }
}

async function send() {
  if (!prompt.value.trim() || sending.value) return;
  sending.value = true;
  error.value = '';
  try {
    await invoke('photo_edit_start', {
      fileId: props.fileId,
      prompt: prompt.value.trim(),
      fromVersion: basedOn.value,
    });
    queued.value = true;
    prompt.value = '';
    await refresh();
  } catch (e) {
    error.value = String(e);
  } finally {
    sending.value = false;
  }
}

function close() {
  emit('close');
}

watch(() => props.fileId, () => {
  basedOn.value = null;
  queued.value = false;
  error.value = '';
  void refresh();
});

onMounted(() => {
  void refresh();
  input.value?.focus();
  // While the dialog is open, keep the versions and the queue current.
  timer = window.setInterval(refresh, 4000);
});
onBeforeUnmount(() => {
  if (timer) window.clearInterval(timer);
});
</script>
