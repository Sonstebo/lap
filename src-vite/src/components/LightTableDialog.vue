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

        <!-- ask for a set; nothing is added until you keep it -->
        <div class="flex gap-2 items-center">
          <input
            v-model="person"
            list="light-table-people"
            class="w-36 shrink-0 rounded bg-base-100/60 border border-base-content/20 px-2 py-1 outline-none"
            placeholder="Person"
            title="Only photos this person's face was found in"
            @keydown.enter.exact.prevent="propose"
          />
          <datalist id="light-table-people">
            <option v-for="p in people" :key="p" :value="p"></option>
          </datalist>
          <input
            v-model="brief"
            class="flex-1 min-w-0 rounded bg-base-100/60 border border-base-content/20 px-2 py-1 outline-none"
            placeholder="What should be in it? For example: the winter trip"
            @keydown.enter.exact.prevent="propose"
          />
          <input
            v-model.number="suggestCount"
            type="number"
            min="2"
            max="60"
            class="w-16 shrink-0 rounded bg-base-100/60 border border-base-content/20 px-2 py-1 outline-none"
            title="How many photos to suggest"
          />
          <button
            class="shrink-0 px-3 py-1 rounded border border-base-content/20 hover:bg-base-content/10 disabled:opacity-40"
            :disabled="asking || (!brief.trim() && !person.trim() && !items.length)"
            :title="askHint"
            @click="propose"
          >{{ asking ? 'Looking…' : askLabel }}</button>
        </div>

        <div v-if="candidates.length" class="flex flex-col gap-1">
          <div class="flex items-center gap-2 text-[11px] uppercase tracking-wider opacity-50">
            <span class="flex-1">Suggested &mdash; {{ candidates.length }} waiting</span>
            <button class="normal-case tracking-normal underline" @click="keepAll">Keep all</button>
            <button class="normal-case tracking-normal underline" @click="candidates = []">Discard</button>
          </div>
          <div class="flex gap-1 overflow-x-auto pb-1">
            <div
              v-for="(c, i) in candidates"
              :key="c.path"
              class="relative shrink-0 w-20 h-16 rounded overflow-hidden border border-base-content/20 group"
            >
              <img :src="thumb(c.file_id)" class="w-full h-full object-cover" :alt="c.filename" />
              <div class="absolute inset-x-0 bottom-0 flex opacity-0 group-hover:opacity-100 transition-opacity">
                <button class="flex-1 py-0.5 text-[10px] font-semibold text-white bg-success/90" @click="keep(i)">Keep</button>
                <button class="flex-1 py-0.5 text-[10px] font-semibold text-white bg-error/90" @click="candidates.splice(i, 1)">Drop</button>
              </div>
            </div>
          </div>
        </div>

        <div v-if="items.length" class="flex gap-1 overflow-x-auto pb-1">
          <div
            v-for="(it, i) in items"
            :key="it.path"
            class="relative shrink-0 w-14 h-11 rounded overflow-hidden border border-base-content/20 group"
            :title="it.path"
          >
            <img :src="thumb(it.file_id)" class="w-full h-full object-cover" alt="" />
            <button
              class="absolute right-0 top-0 w-4 h-4 text-[10px] leading-4 text-white bg-base-300/80 opacity-0 group-hover:opacity-100"
              title="Take it out of the set"
              @click="drop(i)"
            >&times;</button>
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

        <!-- keep the page as a recipe in a book; it is drawn when the book is exported -->
        <div class="flex gap-2 items-center">
          <input
            v-model="bookName"
            list="light-table-books"
            class="w-44 shrink-0 rounded bg-base-100/60 border border-base-content/20 px-2 py-1 outline-none"
            placeholder="Book name"
          />
          <datalist id="light-table-books">
            <option v-for="b in books" :key="b" :value="b"></option>
          </datalist>
          <input
            v-model="pageCaption"
            class="flex-1 min-w-0 rounded bg-base-100/60 border border-base-content/20 px-2 py-1 outline-none"
            placeholder="Caption for this page (optional)"
          />
          <button
            class="shrink-0 px-3 py-1 rounded border border-base-content/20 hover:bg-base-content/10 disabled:opacity-40"
            :disabled="!bookName.trim() || items.length < 2 || adding"
            @click="addToBook"
          >{{ adding ? 'Adding…' : 'Add page to book' }}</button>
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
            <span>Photos</span><span>{{ count }} of {{ items.length }}</span>
          </label>
          <input type="range" min="2" :max="maxCount" v-model.number="count" class="range range-xs" />
        </div>

        <div class="flex flex-col gap-1">
          <label class="text-[11px] uppercase tracking-wider opacity-50 flex justify-between">
            <span>Variety</span><span>{{ variety.toFixed(2) }}</span>
          </label>
          <input type="range" min="0" max="1" step="0.05" v-model.number="variety" class="range range-xs" />
          <p class="text-[10px] leading-tight opacity-40">0 is the closest match to what you asked for, 1 the widest spread.</p>
        </div>

        <div class="flex flex-col gap-1">
          <label class="text-[11px] uppercase tracking-wider opacity-50">Spread over</label>
          <div class="flex flex-wrap gap-1">
            <button
              v-for="sp in spreads"
              :key="sp.id"
              class="px-2 py-1 rounded border text-xs"
              :class="sp.id === spread ? 'bg-primary text-primary-content border-primary' : 'border-base-content/20 hover:bg-base-content/10'"
              @click="spread = sp.id"
            >{{ sp.label }}</button>
          </div>
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
import { getThumbUrl } from '@/common/utils';

const props = defineProps<{ items?: Item[]; collectionLabel?: string }>();
const emit = defineEmits<{ (e: 'close'): void }>();

type Item = { path: string; file_id: number; filename?: string };
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

const items = ref<Item[]>([]);
const candidates = ref<Item[]>([]);
const brief = ref('');
const bookName = ref('');
const pageCaption = ref('');
const books = ref<string[]>([]);
const adding = ref(false);
const added = ref('');
const asking = ref(false);
const variety = ref(0.45);
const spread = ref('none');
const person = ref('');
const people = ref<string[]>([]);
const suggestCount = ref(12);
const spreads = [
  { id: 'none', label: 'Nothing' }, { id: 'day', label: 'Days' },
  { id: 'month', label: 'Months' }, { id: 'year', label: 'Years' },
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

const maxCount = computed(() => Math.max(2, items.value.length));
const busy = computed(() => drafting.value || printing.value);
const busyLabel = computed(() => (printing.value ? 'fetching originals and composing at print size…' : 'laying it out…'));
const title = computed(() =>
  props.collectionLabel ? `Light table · ${props.collectionLabel}` : 'Light table · selected photos');
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
const footer = computed(() => {
  if (added.value) return `${added.value} · export it with: photos book export`;
  if (printedPath.value) return 'printed; the file is in ~/Pictures/Photos Collages';
  return 'nothing is changed until you print or add a page';
});

function thumb(fileId: number): string {
  return getThumbUrl(fileId, false, 256);
}

function args() {
  return {
    collection: null,
    paths: items.value.slice(0, count.value).map((i) => i.path),
    template: template.value,
    shape: shape.value,
    gap: gap.value,
    count: count.value,
    faceSafe: faceSafe.value,
  };
}

async function draft() {
  if (items.value.length < 2) {
    page.value = null;
    error.value = items.value.length
      ? 'one photo is not a collage: press More like this, or describe what should go with it'
      : 'nothing to lay out yet';
    return;
  }
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

async function propose() {
  if (asking.value) return;
  asking.value = true;
  error.value = '';
  try {
    const answer = (await invoke('light_table_select', {
      brief: brief.value,
      // With nothing typed, grow the set around the photo you started from.
      similar: (brief.value.trim() || person.value.trim()) ? null : (items.value[0]?.path ?? null),
      person: person.value.trim() || null,
      // How many to suggest is its own number: capping it at the size of the set
      // on screen meant a set of one could only ever be offered one companion.
      count: Math.max(2, Math.min(60, suggestCount.value || 12)),
      variety: variety.value,
      spread: spread.value,
    })) as { candidates?: Item[]; reason?: string; unmatched?: number };
    candidates.value = answer.candidates ?? [];
    if (!candidates.value.length) {
      error.value = answer.reason || 'nothing matched; try describing it differently';
    } else if (answer.unmatched) {
      error.value = `${answer.unmatched} of the chosen photos are not in this library yet`;
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    asking.value = false;
  }
}

function keep(i: number) {
  const [taken] = candidates.value.splice(i, 1);
  if (taken && !items.value.some((x) => x.path === taken.path)) {
    items.value = [...items.value, taken];
    count.value = items.value.length;
  }
}

function keepAll() {
  const fresh = candidates.value.filter((c) => !items.value.some((x) => x.path === c.path));
  candidates.value = [];
  if (!fresh.length) return;
  items.value = [...items.value, ...fresh];
  count.value = items.value.length;
}

function drop(i: number) {
  items.value = items.value.filter((_, n) => n !== i);
  count.value = Math.max(2, Math.min(count.value, items.value.length));
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

async function addToBook() {
  if (adding.value || !bookName.value.trim()) return;
  adding.value = true;
  error.value = '';
  try {
    const done = (await invoke('light_table_add_to_book', {
      book: bookName.value.trim(),
      shape: shape.value,
      paths: items.value.slice(0, count.value).map((i) => i.path),
      template: template.value,
      caption: pageCaption.value,
    })) as { page: number; book: string; book_created?: boolean };
    added.value = done.book_created
      ? `started ${done.book} with page ${done.page}`
      : `page ${done.page} added to ${done.book}`;
    pageCaption.value = '';
    void loadBooks();
  } catch (e) {
    error.value = String(e);
  } finally {
    adding.value = false;
  }
}

const askLabel = computed(() =>
  (brief.value.trim() || person.value.trim()) ? 'Suggest' : 'More like this');
const askHint = computed(() =>
  person.value.trim() && !brief.value.trim()
    ? `Photos ${person.value.trim()} appears in`
    : brief.value.trim()
      ? 'Find photos matching what you typed'
      : 'Find photos that go with the first one');

async function loadPeople() {
  try {
    const rows = (await invoke('light_table_people')) as { name?: string; display_name?: string; photos?: number }[];
    people.value = rows
      .filter((r) => (r.photos ?? 0) > 0 && (r.name || r.display_name))
      .map((r) => String(r.name || r.display_name));
  } catch {
    people.value = [];
  }
}

async function loadBooks() {
  try {
    const rows = (await invoke('light_table_books')) as { name: string }[];
    books.value = rows.map((r) => r.name);
  } catch {
    books.value = [];
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

watch([template, shape, gap, count, faceSafe, items], () => {
  printedPath.value = '';
  if (items.value.length >= 2) redraw();
}, { deep: true });

onMounted(() => {
  void loadBooks();
  void loadPeople();
  items.value = [...(props.items ?? [])];
  count.value = Math.max(2, Math.min(items.value.length || 9, 12));
  if (items.value.length >= 2) void draft();
});
onBeforeUnmount(() => {
  if (timer) window.clearTimeout(timer);
});
</script>
