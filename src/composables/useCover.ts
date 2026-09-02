import { invoke } from "@tauri-apps/api/core";
import { ref } from "vue";

interface CoverPayload {
  dataUrl: string;
}

/**
 * 提供带过期请求保护的 Rust 封面缓存加载状态。
 */
export const useCover = () => {
  const coverUrl = ref("");
  const loading = ref(false);
  const errorText = ref("");
  let requestId = 0;

  /**
   * 清空当前封面状态并作废尚未完成的旧请求。
   */
  const clearCover = (): void => {
    requestId += 1;
    coverUrl.value = "";
    loading.value = false;
    errorText.value = "";
  };

  /**
   * 根据 SID 从 Rust 磁盘缓存或网络加载封面。
   */
  const loadCover = async (
    beatmapSetId: number | null,
    missing = false
  ): Promise<void> => {
    const currentRequestId = ++requestId;
    coverUrl.value = "";
    errorText.value = "";

    if (missing) {
      errorText.value = "该条目缺少本地谱面信息";
      loading.value = false;
      return;
    }
    if (beatmapSetId == null || beatmapSetId <= 0) {
      errorText.value = "该谱面没有可用的 SID";
      loading.value = false;
      return;
    }

    loading.value = true;
    try {
      const payload = await invoke<CoverPayload | null>("get_cover", {
        beatmapSetId,
      });
      if (currentRequestId !== requestId) return;
      if (!payload?.dataUrl) {
        errorText.value = "暂无可用图片";
        return;
      }
      coverUrl.value = payload.dataUrl;
    } catch (error) {
      if (currentRequestId !== requestId) return;
      errorText.value = error instanceof Error ? error.message : String(error);
    } finally {
      if (currentRequestId === requestId) {
        loading.value = false;
      }
    }
  };

  return { coverUrl, loading, errorText, clearCover, loadCover };
};
