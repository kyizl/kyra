<script setup lang="ts">
import { Badge } from '@renderer/components/ui/badge';
import { Button } from '@renderer/components/ui/button';
import { Dialog } from '@renderer/components/ui/dialog';
import { useStaggerReveal } from '@renderer/composables/useStaggerReveal';
import { useNicksStore } from '@renderer/store/nicks';
import { Pencil, Plus, Tag, Trash2 } from 'lucide-vue-next';
import { ref } from 'vue';

const nicks = useNicksStore();

const modalOpen = ref(false);
const editId = ref<string | null>(null);
const formNick = ref('');
const formReal = ref('');

const listRoot = ref<HTMLElement | null>(null);
useStaggerReveal(listRoot, {
  selector: '.nick-row',
  watchSource: () => nicks.nicks.length,
});

function openAdd(): void {
  editId.value = null;
  formNick.value = '';
  formReal.value = '';
  modalOpen.value = true;
}

function openEdit(nick: { id: string; nick: string; realName: string }): void {
  editId.value = nick.id;
  formNick.value = nick.nick;
  formReal.value = nick.realName;
  modalOpen.value = true;
}

function save(): void {
  if (!formNick.value || !formReal.value) return;
  if (editId.value) nicks.update(editId.value, formNick.value, formReal.value);
  else nicks.add(formNick.value, formReal.value);
  modalOpen.value = false;
}
</script>

<template>
  <div class="animate-fade-in flex h-full flex-col overflow-hidden">
    <div
      class="no-drag nick-banner flex shrink-0 items-center justify-between gap-3 px-3.5 py-2.5"
    >
      <div class="flex min-w-0 items-center gap-2">
        <div
          class="flex h-8 w-8 shrink-0 items-center justify-center rounded-full"
          style="
            background: rgba(var(--color-accent-rgb), 0.12);
            border: 1px solid rgba(var(--color-accent-rgb), 0.28);
          "
        >
          <Tag
            :size="16"
            style="color: var(--color-accent-light)"
          />
        </div>
        <div class="min-w-0">
          <div
            style="
              font-size: 0.85rem;
              font-weight: 600;
              color: var(--color-accent-light);
              line-height: 1.2;
            "
          >
            Nick Manager
          </div>
          <div
            class="truncate"
            style="
              font-size: 0.8rem;
              color: var(--color-ink-3);
              line-height: 1.35;
              margin-top: 1px;
            "
          >
            Map in-game nicks to real usernames.
          </div>
        </div>
      </div>
      <Button
        variant="control"
        size="sm"
        class="shrink-0 gap-1.5"
        @click="openAdd"
      >
        <Plus :size="11" />
        Add Nick
      </Button>
    </div>

    <div
      ref="listRoot"
      class="themed-scroll flex-1 overflow-y-auto"
    >
      <div
        v-if="nicks.nicks.length === 0"
        class="flex h-full flex-col items-center justify-center gap-2.5"
        style="opacity: 0.35"
      >
        <div
          class="glow-halo flex h-10 w-10 items-center justify-center rounded-full"
          style="
            background: var(--color-surface-1);
            border: 1px solid var(--color-border);
          "
        >
          <Tag
            :size="18"
            style="color: var(--color-ink-3)"
          />
        </div>
        <p style="font-size: 0.78rem; color: var(--color-ink-3)">No nicks configured.</p>
      </div>

      <table
        v-else
        class="w-full"
      >
        <thead
          class="sticky top-0 z-10"
          style="background: rgba(var(--color-bg-rgb), 0.97)"
        >
          <tr
            class="border-b"
            style="border-color: var(--color-border)"
          >
            <th class="kyra-eyebrow px-3.5 py-2 text-left">Nick</th>
            <th class="kyra-eyebrow px-3.5 py-2 text-left">Real Name</th>
            <th class="w-16 py-2" />
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="nick in nicks.nicks"
            :key="nick.id"
            class="nick-row group glass-row border-b"
            style="border-color: rgba(var(--color-accent-rgb), 0.07)"
          >
            <td class="px-3.5 py-2.5">
              <Badge
                variant="nick"
                class="px-2.5 text-[0.74rem] font-extrabold tracking-normal"
              >
                {{ nick.nick }}
              </Badge>
            </td>
            <td
              class="px-3.5 py-2.5 font-medium"
              style="font-size: 0.82rem; color: var(--color-ink-1)"
            >
              {{ nick.realName }}
            </td>
            <td class="px-2.5 py-2.5">
              <div
                class="no-drag flex items-center justify-end gap-1 opacity-0 transition-opacity group-hover:opacity-100"
              >
                <Button
                  variant="ghost"
                  size="icon-sm"
                  @click="openEdit(nick)"
                >
                  <Pencil :size="10" />
                </Button>
                <Button
                  variant="destructive"
                  size="icon-sm"
                  @click="nicks.remove(nick.id)"
                >
                  <Trash2 :size="10" />
                </Button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <Dialog
      v-model:open="modalOpen"
      :title="editId ? 'Edit Nick' : 'Add Nick'"
      width="280px"
    >
      <div class="no-drag flex flex-col gap-3">
        <div class="flex flex-col gap-1.5">
          <label style="font-size: 0.76rem; color: var(--color-ink-3)"
            >In-game Nick</label
          >
          <input
            v-model.trim="formNick"
            class="input-field"
            placeholder="Nicked username"
            @keydown.enter="save"
          />
        </div>
        <div class="flex flex-col gap-1.5">
          <label style="font-size: 0.76rem; color: var(--color-ink-3)"
            >Real Username</label
          >
          <input
            v-model.trim="formReal"
            class="input-field"
            placeholder="Actual username"
            @keydown.enter="save"
          />
        </div>
      </div>

      <div class="no-drag flex justify-end gap-2">
        <Button
          variant="control"
          size="sm"
          @click="modalOpen = false"
        >
          Cancel
        </Button>
        <Button
          variant="control"
          size="sm"
          :disabled="!formNick || !formReal"
          @click="save"
        >
          Save
        </Button>
      </div>
    </Dialog>
  </div>
</template>

<style scoped>
.nick-banner {
  position: relative;
  overflow: hidden;
  border-bottom: 1px solid rgba(var(--color-accent-rgb), 0.18);
}
.nick-banner::before {
  content: '';
  position: absolute;
  inset: 0;
  width: 300%;
  left: -100%;
  background: linear-gradient(
    105deg,
    rgba(var(--color-accent-rgb), 0.07) 0%,
    rgba(var(--color-accent-rgb), 0.03) 50%,
    rgba(var(--color-accent-rgb), 0.05) 100%
  );
  will-change: transform;
  animation: banner-shimmer 6s ease-in-out infinite;
  animation-play-state: var(--anim-play-state, running);
  pointer-events: none;
}
</style>
