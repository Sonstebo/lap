<template>
  <ModalDialog :title="title" :width="860" :height="620" @cancel="close">
    <div class="flex flex-col h-full gap-2 p-3 pt-0 text-sm">
      <!-- the picture, as it is now -->
      <div class="relative flex-1 min-h-0 rounded bg-base-100/40 flex items-center justify-center overflow-hidden">
        <img v-if="shownSrc" :src="shownSrc" class="max-w-full max-h-full object-contain" :alt="shownLabel" />
        <div v-else class="opacity-50">no preview</div>
        <div
          v-if="busy"
          class="absolute inset-x-0 bottom-0 px-3 py-2 bg-base-300/80 backdrop-blur-sm flex items-center gap-2"
        >
          <span class="loading loading-spinner loading-xs"></span>
          <span class="truncate">{{ busyLabel }}</span>
        </div>
      </div>

      <!-- what it was asked, for the version on screen -->
      <p class="h-5 truncate opacity-70">
        <span v-if="shown">{{ shown.prompt }} · {{ shown.method === 'generated' ? 'generated' : 'adjusted' }}<span v-if="shownSize"> · {{ shownSize }}</span></span>
        <span v-else>The original, straight from iCloud. It is never changed.</span>
      </p>

      <!-- every version, oldest first; click one to see it and build on it -->
      <div class="flex gap-2 overflow-x-auto pb-1">
        <button
          class="shrink-0 w-16 h-16 rounded overflow-hidden border-2"
          :class="selected === null ? 'border-primary' : 'border-transparent opacity-70 hover:opacity-100'"
          title="The original"
          @click="selected = null"
        >
          <img v-if="originalSrc" :src="originalSrc" class="w-full h-full object-cover" alt="original" />
        </button>
        <button
          v-for="v in versions"
          :key="v.id"
          class="shrink-0 w-16 h-16 rounded overflow-hidden border-2 relative"
          :class="selected === v.id ? 'border-primary' : 'border-transparent opacity-70 hover:opacity-100'"
          :title="v.prompt"
          @click="selected = v.id"
        >
          <img v-if="v.exists" :src="fileSrc(v.path)" class="w-full h-full object-cover" :alt="v.prompt" />
          <span v-else class="absolute inset-0 grid place-items-center opacity-60">gone</span>
        </button>
        <div
          v-for="j in running"
          :key="'job' + j.id"
          class="shrink-0 w-16 h-16 rounded border-2 border-dashed border-base-content/30 grid place-items-center"
          :title="j.prompt"
        >
          <span class="loading loading-spinner loading-xs"></span>
        </div>
      </div>

      <!-- ask for the next change -->
      <div class="flex flex-wrap gap-1">
        <button
          v-for="p in presets"
          :key="p.label"
          class="px-2 py-1 rounded border border-base-content/20 hover:bg-base-content/10"
          @click="ask(p.prompt)"
        >{{ p.label }}</button>
      </div>

      <div class="flex gap-2 items-end">
        <textarea
          ref="input"
          v-model="prompt"
          rows="2"
          class="flex-1 rounded bg-base-100/60 border border-base-content/20 p-2 outline-none resize-none"
          :placeholder="placeholder"
          @keydown.enter.exact.prevent="ask()"
          @focus="uiStore.pushInputHandler('ask-photo')"
          @blur="uiStore.popInputHandler()"
        ></textarea>
        <button
          class="px-3 py-2 rounded bg-primary text-primary-content disabled:opacity-40"
          :disabled="!prompt.trim() || sending"
          @click="ask()"
        >Ask</button>
      </div>

      <p class="h-4 text-xs truncate" :class="error ? 'text-error' : 'opacity-50'">
        {{ error || footer }}
      </p>
    </div>
  </ModalDialog>
</template>

<script setup lang="ts">
/**
 * Ask about this photo.
 *
 * The picture is on screen; you type what should change; the new version
 * appears in the strip and on screen when it is ready. Whatever is selected is
 * what the next request builds on, so "warmer" then "now black and white"
 * behaves the way a conversation does. The original is never modified: every
 * version is a new file, and the work runs in the background.
 */
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import ModalDialog from '@/components/ModalDialog.vue';
import { useUIStore } from '@/stores/uiStore';
import { getPreviewUrl, getThumbUrl } from '@/common/utils';

const props = defineProps<{ fileId: number; fileName?: string }>();
const emit = defineEmits<{ (e: 'close'): void }>();
const uiStore = useUIStore();

type Version = { id: number; prompt: string; method?: string; path: string; exists: boolean; width?: number; height?: number };
type Job = { id: number; state: string; prompt: string; asset_id?: string };

const versions = ref<Version[]>([]);
const jobs = ref<Job[]>([]);
const selected = ref<number | null>(null);
const prompt = ref('');
const sending = ref(false);
const error = ref('');
const input = ref<HTMLTextAreaElement | null>(null);
let timer: number | undefined;

const presets = [
  { label: 'Restore', prompt: 'Restore this old photo: repair damage, remove scratches and noise, and make it sharp, keeping the same people and framing.' },
  { label: 'Enhance', prompt: 'Improve exposure, contrast and colour naturally, without changing the content.' },
  { label: 'Black and white', prompt: 'Convert to black and white with good contrast.' },
  { label: 'Straighten', prompt: 'Straighten the horizon and crop to the largest sensible rectangle.' },
  { label: 'Remove background', prompt: 'Remove the background, keeping the main subject, on a plain white background.' },
];

const title = computed(() => (props.fileName ? `Ask about ${props.fileName}` : 'Ask about this photo'));
const shown = computed(() => versions.value.find((v) => v.id === selected.value) ?? null);
const originalSrc = computed(() => getThumbUrl(props.fileId, false, 512));
const shownSrc = computed(() =>
  shown.value ? fileSrc(shown.value.path) : getPreviewUrl(props.fileId, '', false, 0, 'processed'),
);
const shownLabel = computed(() => shown.value?.prompt ?? 'original');
const shownSize = computed(() => (shown.value?.width ? `${shown.value.width}x${shown.value.height}` : ''));
const running = computed(() => jobs.value.filter((j) => j.state === 'queued' || j.state === 'running'));
const busy = computed(() => running.value.length > 0 || sending.value);
const busyLabel = computed(() =>
  sending.value ? 'sending…' : running.value[0]?.prompt ?? '',
);
const placeholder = computed(() =>
  selected.value ? 'Change this version…' : 'What should change? For example: remove the plate, restore this old photo, warmer light',
);
const footer = computed(() =>
  `${selected.value ? `building on version ${selected.value}` : 'building on the original'} · results in ~/Pictures/Photos Edits · Enter to ask`,
);

function fileSrc(path: string): string {
  return convertFileSrc(path);
}

async function refresh() {
  try {
    const v = (await invoke('photo_versions', { fileId: props.fileId })) as { versions: Version[] };
    const known = new Set(versions.value.map((x) => x.id));
    versions.value = v.versions ?? [];
    // Show a version the moment it appears, which is the point of asking.
    const fresh = versions.value.find((x) => !known.has(x.id));
    if (fresh && known.size) selected.value = fresh.id;
  } catch (e) {
    error.value = String(e);
  }
  try {
    const j = (await invoke('photo_edit_jobs', { limit: 12 })) as { jobs: Job[] };
    jobs.value = j.jobs ?? [];
  } catch {
    jobs.value = [];
  }
}

async function ask(text?: string) {
  const wanted = (text ?? prompt.value).trim();
  if (!wanted || sending.value) return;
  sending.value = true;
  error.value = '';
  try {
    await invoke('photo_edit_start', { fileId: props.fileId, prompt: wanted, fromVersion: selected.value });
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
  selected.value = null;
  error.value = '';
  void refresh();
});

onMounted(() => {
  void refresh();
  input.value?.focus();
  timer = window.setInterval(refresh, 3000);
});
onBeforeUnmount(() => {
  if (timer) window.clearInterval(timer);
});
</script>
