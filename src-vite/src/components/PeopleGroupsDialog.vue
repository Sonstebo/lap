<template>
  <ModalDialog title="Who is this?" :width="960" :height="700" @cancel="close">
    <div class="flex flex-col h-full gap-3 p-3 pt-0 text-sm">
      <p class="opacity-70">
        Faces that chain together through the ages between them are one group, so naming a
        group names a whole life at once. A face that already names someone else is left alone.
      </p>

      <div class="flex gap-2 items-center flex-wrap">
        <button class="px-3 py-1 rounded border border-base-content/20 hover:bg-base-content/10 disabled:opacity-40"
                :disabled="busy" @click="rebuild">
          {{ building ? 'Grouping…' : 'Find groups' }}
        </button>
        <label class="flex items-center gap-2 cursor-pointer">
          <input type="checkbox" v-model="unnamedOnly" class="checkbox checkbox-xs" @change="load" />
          <span class="text-xs">Only groups without a name</span>
        </label>
        <label class="flex items-center gap-2 cursor-pointer">
          <span class="text-xs opacity-60">At least</span>
          <input type="number" min="2" max="500" v-model.number="minSize"
                 class="w-16 rounded bg-base-100/60 border border-base-content/20 px-2 py-0.5 outline-none"
                 @change="load" />
          <span class="text-xs opacity-60">faces</span>
        </label>
        <span class="flex-1"></span>
        <button class="px-3 py-1 rounded border border-base-content/20 hover:bg-base-content/10 disabled:opacity-40"
                :disabled="busy" title="Spread the names you gave over the faces nobody named by hand"
                @click="rematch">{{ spreading ? 'Spreading…' : 'Apply to the rest' }}</button>
      </div>

      <div class="flex-1 min-h-0 overflow-y-auto flex flex-col gap-2 pr-1">
        <div v-if="!groups.length && !busy" class="opacity-50 py-6 text-center">
          No groups yet. Press Find groups.
        </div>
        <div
          v-for="g in groups"
          :key="g.cluster"
          class="rounded border border-base-content/15 bg-base-100/30 p-2 flex gap-3 items-center"
        >
          <div class="flex gap-1 shrink-0">
            <img
              v-for="fid in g.sample_faces.slice(0, 5)"
              :key="fid"
              :src="crops[fid] || ''"
              class="w-14 h-14 rounded object-cover bg-base-300"
              :alt="'face ' + fid"
            />
          </div>
          <div class="flex-1 min-w-0">
            <div class="font-medium truncate">
              {{ g.person || 'Not named yet' }}
              <span class="opacity-50 font-normal">· {{ g.faces.toLocaleString() }} faces</span>
            </div>
            <div class="text-xs opacity-50">
              {{ (g.first_seen || '').slice(0, 7) }} to {{ (g.last_seen || '').slice(0, 7) }}
              <span v-if="g.named_faces"> · {{ g.named_faces }} already named</span>
            </div>
          </div>
          <input
            v-model="names[g.cluster]"
            list="people-known"
            class="w-48 shrink-0 rounded bg-base-100/60 border border-base-content/20 px-2 py-1 outline-none"
            :placeholder="g.person || 'Who is this?'"
            @keydown.enter.exact.prevent="nameGroup(g)"
          />
          <button
            class="shrink-0 px-3 py-1 rounded bg-primary text-primary-content disabled:opacity-40"
            :disabled="busy || !(names[g.cluster] || '').trim()"
            @click="nameGroup(g)"
          >Name</button>
        </div>
      </div>
      <datalist id="people-known">
        <option v-for="p in known" :key="p" :value="p"></option>
      </datalist>

      <p class="h-4 text-xs truncate" :class="error ? 'text-error' : 'opacity-50'">
        {{ error || status }}
      </p>
    </div>
  </ModalDialog>
</template>

<script setup lang="ts">
/**
 * Who is this?
 *
 * Recognition compares a face against reference faces, which cannot span a
 * childhood. Grouping can: faces are joined to their neighbours and the joins
 * are transitive, so a baby and a teenager end up in one group by way of every
 * age between. Naming that group names all of it and seeds the recogniser at
 * every age, which is what makes the rest of the library follow.
 *
 * Every name is written by the album's tool, where the names live, so nothing
 * said here is lost the next time the library is refreshed.
 */
import { ref, computed, onMounted } from 'vue';
import { invoke, convertFileSrc } from '@tauri-apps/api/core';
import ModalDialog from '@/components/ModalDialog.vue';

const emit = defineEmits<{ (e: 'close'): void }>();

type Group = {
  cluster: number; faces: number; person: string | null; person_id: string | null;
  named_faces: number; first_seen: string | null; last_seen: string | null; sample_faces: number[];
};

const groups = ref<Group[]>([]);
const crops = ref<Record<number, string>>({});
const names = ref<Record<number, string>>({});
const known = ref<string[]>([]);
const unnamedOnly = ref(true);
const minSize = ref(4);
const loading = ref(false);
const building = ref(false);
const spreading = ref(false);
const naming = ref(false);
const error = ref('');
const status = ref('');

const busy = computed(() => loading.value || building.value || spreading.value || naming.value);

async function load() {
  loading.value = true;
  error.value = '';
  try {
    const answer = (await invoke('people_groups', {
      limit: 40, unnamedOnly: unnamedOnly.value, minSize: Math.max(2, minSize.value || 2),
    })) as { clusters: Group[] };
    groups.value = answer.clusters ?? [];
    status.value = `${groups.value.length} groups`;
    await loadCrops();
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function loadCrops() {
  const wanted = groups.value.flatMap((g) => g.sample_faces.slice(0, 5)).filter((f) => !crops.value[f]);
  if (!wanted.length) return;
  try {
    // in batches, so one slow photo does not hold up the whole list
    for (let i = 0; i < wanted.length; i += 40) {
      const made = (await invoke('people_face_crops', {
        faceIds: wanted.slice(i, i + 40), size: 160,
      })) as { face_id: number; path: string | null }[];
      const next = { ...crops.value };
      for (const m of made) {
        if (m.path) next[m.face_id] = convertFileSrc(m.path);
      }
      crops.value = next;
    }
  } catch {
    // a missing face picture is not worth an error message
  }
}

async function rebuild() {
  building.value = true;
  error.value = '';
  status.value = 'grouping every face; this takes a moment';
  try {
    const done = (await invoke('people_build_groups', {})) as
      { clusters: number; grouped: number; faces: number };
    status.value = `${done.clusters} groups over ${done.grouped} of ${done.faces} faces`;
    await load();
  } catch (e) {
    error.value = String(e);
  } finally {
    building.value = false;
  }
}

async function nameGroup(g: Group) {
  const who = (names.value[g.cluster] || '').trim();
  if (!who) return;
  naming.value = true;
  error.value = '';
  try {
    const done = (await invoke('people_name_group', {
      cluster: g.cluster, person: who, force: false,
    })) as { faces: number; left_alone: number; seeds: number; first: string; last: string };
    status.value = `named ${done.faces} faces ${who}, seeded ${done.seeds} across ` +
      `${(done.first || '').slice(0, 4)} to ${(done.last || '').slice(0, 4)}` +
      (done.left_alone ? `; ${done.left_alone} kept the name they had` : '');
    if (!known.value.includes(who)) known.value = [...known.value, who];
    names.value = { ...names.value, [g.cluster]: '' };
    await load();
  } catch (e) {
    error.value = String(e);
  } finally {
    naming.value = false;
  }
}

async function rematch() {
  spreading.value = true;
  error.value = '';
  try {
    const done = (await invoke('people_rematch')) as { rematch?: { faces: number; changed: number } };
    const r = done.rematch;
    status.value = r ? `${r.changed} more faces took a name` : 'done';
  } catch (e) {
    error.value = String(e);
  } finally {
    spreading.value = false;
  }
}

async function loadKnown() {
  try {
    const rows = (await invoke('light_table_people')) as { name?: string; photos?: number }[];
    known.value = rows.filter((r) => r.name).map((r) => String(r.name));
  } catch {
    known.value = [];
  }
}

function close() {
  emit('close');
}

onMounted(() => {
  void loadKnown();
  void load();
});
</script>
