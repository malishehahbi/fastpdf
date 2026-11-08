import { ref, shallowRef, watch, type Ref } from 'vue';
import { FastPdfDocument } from '@fastpdf/core';

export function usePdfDocument(url: Ref<string> | string) {
  const doc = shallowRef<FastPdfDocument | null>(null);
  const error = ref<string | null>(null);
  const loading = ref(false);

  const src = typeof url === 'string' ? ref(url) : url;

  watch(
    src,
    async (value) => {
      loading.value = true;
      error.value = null;
      try {
        doc.value = await FastPdfDocument.openUrl(value);
      } catch (e) {
        error.value = e instanceof Error ? e.message : String(e);
        doc.value = null;
      } finally {
        loading.value = false;
      }
    },
    { immediate: true },
  );

  return { doc, error, loading };
}
