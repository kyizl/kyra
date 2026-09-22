import type { Nick } from '@renderer/types';
import { nanoid } from '@renderer/composables/nanoid';
import { defineStore } from 'pinia';
import 'pinia-plugin-persistedstate';

export const useNicksStore = defineStore('nicks', {
  state: () => ({ nicks: [] as Nick[] }),

  actions: {
    resolve(name: string): string {
      const lowerName = name.toLowerCase();
      return (
        this.nicks.find((entry) => entry.nick.toLowerCase() === lowerName)?.realName ??
        name
      );
    },
    add(nick: string, realName: string): void {
      if (this.nicks.some((entry) => entry.nick.toLowerCase() === nick.toLowerCase()))
        return;
      this.nicks.push({ id: nanoid(), nick, realName });
    },
    remove(id: string): void {
      const index = this.nicks.findIndex((entry) => entry.id === id);
      if (index !== -1) this.nicks.splice(index, 1);
    },
    update(id: string, nick: string, realName: string): void {
      const entry = this.nicks.find((candidate) => candidate.id === id);
      if (entry) {
        entry.nick = nick;
        entry.realName = realName;
      }
    },
  },

  persist: true,
});
