<template>
  <ModalDialog :title="title" :width="1080" :height="740" @cancel="close">
    <div class="flex gap-3 h-full p-3 pt-0 text-sm">
      <!-- the page, as it will print -->
      <div class="flex-1 min-w-0 flex flex-col gap-2">
        <div class="relative flex-1 min-h-0 rounded bg-base-100/40 flex items-center justify-center overflow-hidden">
          <img v-if="pageSrc" :src="pageSrc" class="max-w-full max-h-full object-contain shadow-lg" :alt="alt" />
          <div v-else-if="!error" class="opacity-50">laying it out&hellip;</div>
          <div v-else class="opacity-50">no page</div>
          <div
            v-if="busy"
            class="absolute inset-x-0 bottom-0 px-3 py-2 bg-base-300/80 backdrop-blur-sm flex items-center gap-2"
          >
            <span class="loading loading-spinner loading-xs"></span>
            <span class="truncate">{{ busyLabel }}</span>
          </div>
        </div>

        <div class="h-6 flex items-center gap-2">
          <p class="truncate opacity-70 flex-1">{{ caption }}</p>
          <button
            class="shrink-0 px-2 py-1 rounded border border-base-content/20 hover:bg-base-content/10 disabled:opacity-40"
            :disabled="!printedPath"
            :title="printedPath || 'print it first'"
            @click="revealPrinted"
          >Show in files</button>
        </div>
      </div>

      <!-- the rail -->
      <div class="w-56 shrink-0 flex flex-col gap-3 overflow-y-auto">
        <div class="flex flex-col gap-1">
          <label class="text-[11px] uppercase tracking-wider opacity-50">Template</label>
          <div class="flex flex-wrap gap-1">
            <button
              v-for="t in templates"
              :key="t.id"
              class="px-2 py-1 rounded border text-xs"
              :class="t.id === template ? 'bg-primary text-primary-content border-primary' : 'border-base-content/20 hover:bg-base-content/10'"
              :title="t.hint"
              @click="template = t.id"
            >{{ t.label }}</button>
          </div>
        </div>

        <div class="flex flex-col gap-1">
          <label class="text-[11px] uppercase tracking-wider opacity-50">Shape</label>
          <div class="flex flex-wrap gap-1">
            <button
              v-for="s in shapes"
              :key="s.id"
              class="px-2 py-1 rounded border text-xs"
              :class="s.id === shape ? 'bg-primary text-primary-content border-primary' : 'border-base-content/20 hover:bg-base-content/10'"
              @click="shape = s.id"
            >{{ s.label }}</button>
          </div>
        </div>

        <div class="flex flex-col gap-1">
          <label class="text-[11px] uppercase tracking-wider opacity-50 flex justify-between">
            <span>Photos</span><span>{{ count }}</span>
          </label>
          <input type="range" min="2" :max="maxCount" v-model.number="count" class="range range-xs" />
        </div>

        <div class="flex flex-col gap-1">
          <label class="text-[11px] uppercase tracking-wider opacity-50 flex justify-between">
            <span>Gap</span><span>{{ gap }}</span>
          </label>
          <input type="range" min="0" max="40" v-model.number="gap" class="range range-xs" />
        </div>

        <label class="flex items-center gap-2 cursor-pointer">
          <input type="checkbox" v-model="faceSafe" class="checkbox checkbox-xs" />
          <span class="text-xs">Keep faces in frame</span>
        </label>

        <div class="flex flex-col gap-2 mt-1">
          <button
            class="px-3 py-2 rounded bg-primary text-primary-content text-sm font-medium disabled:opacity-40"
            :disabled="busy"
            @click="printIt"
          >Print quality</button>
          <p class="text-[11px] leading-snug opacity-50">
            The draft uses photos already on this computer. Print quality fetches the
            originals from iCloud, composes at 300&nbsp;dpi and releases them again.
          </p>
        </div>

        <p class="text-[11px] leading-snug mt-auto" :class="error ? 'text-error' : 'opacity-50'">
          {{ error || footer }}
        </p>
      </div>
    </div>
  </ModalDialog>
</template>

<script setup lang="ts">
/**
 * The light table.
 *
 * A set of photographs becomes one picture: a collage, a book spread, a contact
 * strip. The picture on screen is drawn by the same code that draws the printed
 * file, at a smaller size, so what is approved here is what comes out. Moving a
 * control re-lays the page in a fraction of a second because it uses the
 * thumbnails already on this computer; only Print quality reaches for iCloud.
 */
import { ref, computed, onMounted, onBeforeUnmount, watch } from 'vue';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import ModalDialog from '@/components/ModalDialog.vue';

const props = defineProps<{ collection?: string | null; paths?: string[]; collectionLabel?: string }>();
const emit = defineEmits<{ (e: 'close'): void }>();

type Page = {
  path: string; width: number; height: number; photos: number; note: string;
  draft: boolean; faces_used?: number; bytes: number; seconds: number; fetched?: number;
};

const templates = [
  { id: 'justified', label: 'Justified', hint: 'Rows that each span the full width' },
  { id: 'grid', label: 'Grid', hint: 'Uniform cells' },
  { id: 'hero', label: 'Hero', hint: 'The first photo large' },
  { id: 'filmstrip', label: 'Strip', hint: 'Equal frames across the page' },
  { id: 'spread', label: 'Spread', hint: 'Two facing pages' },
  { id: 'scatter', label: 'Scatter', hint: 'Mounted prints on a table' },
];
const shapes = [
  { id: '3:2', label: '3:2' }, { id: 'square', label: 'Square' },
  { id: 'a4-landscape', label: 'A4 wide' }, { id: 'a4-portrait', label: 'A4 tall' },
  { id: 'spread', label: 'Two pages' }, { id: '16:9', label: 'Screen' },
];

const template = ref('justified');
const shape = ref('3:2');
const gap = ref(8);
const count = ref(9);
const faceSafe = ref(true);
const page = ref<Page | null>(null);
const printedPath = ref('');
const drafting = ref(false);
const printing = ref(false);
const error = ref('');
let slot = 0;
let timer: number | undefined;

const maxCount = computed(() => Math.max(2, props.paths?.length || 24));
const busy = computed(() => drafting.value || printing.value);
const busyLabel = computed(() => (printing.value ? 'fetching originals and composing at print size…' : 'laying it out…'));
const title = computed(() => {
  const what = props.collectionLabel || props.collection;
  return what ? `Light table · ${what}` : 'Light table · selected photos';
});
const pageSrc = computed(() => (page.value ? convertFileSrc(page.value.path) : ''));
const alt = computed(() => `${template.value} layout of ${page.value?.photos ?? 0} photos`);
const caption = computed(() => {
  const p = page.value;
  if (!p) return '';
  const faces = p.faces_used ? `, ${p.faces_used} with faces kept whole` : '';
  return p.draft
    ? `${p.photos} photos${faces} · draft · ${p.note}`
    : `${p.photos} photos${faces} · ${p.width}×${p.height} · ${p.note}`;
});
const footer = computed(() =>
  printedPath.value ? 'printed; the file is in ~/Pictures/Photos Collages' : 'nothing is changed until you print');

function args() {
  return {
    collection: props.collection ?? null,
    paths: props.collection ? null : (props.paths ?? []),
    template: template.value,
    shape: shape.value,
    gap: gap.value,
    count: count.value,
    faceSafe: faceSafe.value,
  };
}

async function draft() {
  drafting.value = true;
  error.value = '';
  try {
    // Alternate the file so the webview never shows a cached copy of the last page.
    slot = slot ? 0 : 1;
    page.value = (await invoke('light_table_draft', { ...args(), slot, longEdge: 1400 })) as Page;
  } catch (e) {
    error.value = String(e);
  } finally {
    drafting.value = false;
  }
}

function redraw() {
  if (timer) window.clearTimeout(timer);
  timer = window.setTimeout(draft, 220);
}

async function printIt() {
  printing.value = true;
  error.value = '';
  try {
    const done = (await invoke('light_table_render', args())) as Page;
    printedPath.value = done.path;
    page.value = done;
  } catch (e) {
    error.value = String(e);
  } finally {
    printing.value = false;
  }
}

async function revealPrinted() {
  if (!printedPath.value) return;
  try {
    await invoke('reveal_path', { path: printedPath.value });
  } catch (e) {
    error.value = String(e);
  }
}

function close() {
  emit('close');
}

watch([template, shape, gap, count, faceSafe], () => {
  printedPath.value = '';
  redraw();
});
watch(() => [props.collection, props.paths], () => { printedPath.value = ''; redraw(); });

onMounted(() => {
  if (props.paths?.length) count.value = Math.min(props.paths.length, 12);
  void draft();
});
onBeforeUnmount(() => {
  if (timer) window.clearTimeout(timer);
});
</script>
