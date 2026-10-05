import { onBeforeUnmount, onMounted, ref, type Ref } from "vue";

/** Reactive wrapper around `window.matchMedia`. */
export function useMediaQuery(query: string): Ref<boolean> {
  const matches = ref(false);
  let media: MediaQueryList | undefined;

  const update = () => {
    matches.value = media?.matches ?? false;
  };

  onMounted(() => {
    if (typeof window === "undefined" || typeof window.matchMedia !== "function") {
      return;
    }
    media = window.matchMedia(query);
    update();
    media.addEventListener("change", update);
  });

  onBeforeUnmount(() => {
    media?.removeEventListener("change", update);
  });

  return matches;
}
