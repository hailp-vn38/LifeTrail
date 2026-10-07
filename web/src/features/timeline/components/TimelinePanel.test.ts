import { mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { nextTick } from 'vue';
import { expect, it, vi } from 'vitest';
import TimelinePanel from './TimelinePanel.vue';
import { tripView, openTrip } from '../../../test/fixtures/trips';
import { openStop } from "../../../test/fixtures/stationary";
import { usePlaybackStore } from '../../../stores/playback.store';
import { useMapStore } from '../../../stores/map.store';

it('seeks from a timeline card and fades future events from the same playback cursor', async () => {
  const pinia = createPinia();
  setActivePinia(pinia);
  const view = tripView();
  const stop = view.timeline!.find(activity => activity.kind === "stop")!;
  const playback = usePlaybackStore();
  const start = Date.parse('2026-10-04T17:00:00Z');
  playback.initialize({ startTimeMs: start, endTimeMs: start + 86_400_000 });
  const scroll = vi.fn();
  Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', { configurable: true, value: scroll });
  const wrapper = mount(TimelinePanel, { props: { dailyView: view }, global: { plugins: [pinia] } });
  try {
    expect(wrapper.get('.timeline-item--stop').classes()).toContain('is-future');
    await wrapper.get('.timeline-item--stop').trigger('click');
    expect(useMapStore().selectedEventId).toBe(openStop.id);
    expect(playback.seekRequest?.epochMs).toBe(Date.parse(stop.visible_from_at));
    playback.setFrame('paused', Date.parse(stop.visible_from_at) - start);
    await nextTick();
    expect(wrapper.get('.timeline-item--stop').classes()).not.toContain('is-future');
    expect(wrapper.get('.timeline-item--trip').classes()).toContain('is-future');
    const request = playback.seekRequest;
    useMapStore().selectEvent(openTrip.id);
    await nextTick();
    expect(playback.seekRequest).toEqual(request);
    expect(wrapper.get('.timeline-item--trip').attributes('aria-pressed')).toBe('true');
    playback.setFrame('paused', 0);
    await nextTick();
    expect(wrapper.get('.timeline-item--stop').classes()).toContain('is-future');
  } finally {
    wrapper.unmount();
    Reflect.deleteProperty(HTMLElement.prototype, 'scrollIntoView');
  }
});

it('follows the playback clock across Stops and Trips without changing manual selection', async () => {
  const pinia = createPinia();
  setActivePinia(pinia);
  const view = tripView();
  const stop = view.timeline!.find(activity => activity.kind === 'stop')!;
  const playback = usePlaybackStore();
  const start = Date.parse('2026-10-04T17:00:00Z');
  playback.initialize({ startTimeMs: start, endTimeMs: start + 86_400_000 });
  const scroll = vi.fn();
  Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', { configurable: true, value: scroll });
  const wrapper = mount(TimelinePanel, { props: { dailyView: view }, global: { plugins: [pinia] } });
  try {
    playback.setFrame('playing', Date.parse(stop.visible_from_at) - start);
    await nextTick();
    expect(wrapper.get('.timeline-item--stop').attributes('aria-current')).toBe('step');
    expect(useMapStore().selectedEventId).toBeNull();
    playback.setFrame('playing', Date.parse(stop.visible_until_at) - start);
    await nextTick();
    expect(wrapper.find('[aria-current="step"]').exists()).toBe(false);
    playback.setFrame('playing', Date.parse(openTrip.visible_from_at) - start);
    await nextTick();
    expect(wrapper.get('.timeline-item--trip').attributes('aria-current')).toBe('step');
    await nextTick();
    expect(scroll).toHaveBeenCalled();
    playback.setFrame('paused', Date.parse(stop.visible_from_at) - start);
    await nextTick();
    expect(wrapper.get('.timeline-item--stop').attributes('aria-current')).toBe('step');
  } finally {
    wrapper.unmount();
    Reflect.deleteProperty(HTMLElement.prototype, 'scrollIntoView');
  }
});
