import { nextTick, onBeforeUnmount, type Ref } from "vue";

interface ElementTableColumn {
  id?: string;
  property?: string;
  width?: number;
  realWidth?: number;
}

interface ElementTableExpose {
  $el?: HTMLElement;
  columns?: ElementTableColumn[];
  store?: {
    scheduleLayout?: (needUpdateColumns: boolean, immediate?: boolean) => void;
  };
}

interface ResizeSession {
  pointerId: number;
  column: ElementTableColumn;
  columnKey: string;
  startX: number;
  startWidth: number;
  minWidth: number;
  header: HTMLElement;
  pendingWidth: number;
  frameId: number | null;
}

const RESIZE_HANDLE_WIDTH = 8; // 与 Element Plus 默认表头拖拽热区保持一致。
const SUPPRESS_CLICK_TIMEOUT = 250; // 仅在拖拽后的短时间内拦截同一手柄产生的合成点击。

/**
 * 为 Element Plus 表格提供拖动过程中实时反馈的列宽调整能力。
 */
export const useRealtimeColumnResize = (
  tableRef: Ref<ElementTableExpose | undefined>,
  getMinimumWidth: (property: string) => number
) => {
  let session: ResizeSession | null = null;
  let suppressNextClick = false;
  let suppressedClickHeader: HTMLElement | null = null;
  let suppressClickTimer: number | null = null;
  const runtimeWidths = new Map<string, number>(); // 列宽只保存在当前运行会话，不写入持久化设置。

  /**
   * 清理拖拽结束后用于拦截合成点击的临时状态。
   */
  const clearSuppressedClick = (): void => {
    suppressNextClick = false;
    suppressedClickHeader = null;
    if (suppressClickTimer != null) {
      window.clearTimeout(suppressClickTimer);
      suppressClickTimer = null;
    }
  };

  /**
   * 记录拖拽手柄可能触发的合成点击，并在无点击时自动释放状态。
   */
  const armSuppressedClick = (header: HTMLElement): void => {
    clearSuppressedClick();
    suppressNextClick = true;
    suppressedClickHeader = header;
    suppressClickTimer = window.setTimeout(() => {
      suppressNextClick = false;
      suppressedClickHeader = null;
      suppressClickTimer = null;
    }, SUPPRESS_CLICK_TIMEOUT);
  };

  /**
   * 返回列在当前表格中的稳定字段键，供运行时宽度缓存使用。
   */
  const getColumnKey = (column: ElementTableColumn): string =>
    column.property ?? column.id ?? "";

  /**
   * 根据表头元素查找 Element Plus 内部维护的列配置。
   */
  const findColumn = (header: HTMLElement): ElementTableColumn | undefined => {
    const columns = tableRef.value?.columns ?? [];
    return columns.find((column) =>
      column.id ? header.classList.contains(column.id) : false
    );
  };

  /**
   * 立即让 Element Plus 根据当前列宽重新计算表头、单元格和滚动区域。
   */
  const refreshTableLayout = (): void => {
    tableRef.value?.store?.scheduleLayout?.(false, true); // 使用表格内部即时布局入口，确保拖动帧内同步反馈。
  };

  /**
   * 直接同步所有表头和表体 colgroup，保证当前动画帧立刻呈现目标列宽。
   */
  const syncColumnElements = (column: ElementTableColumn, width: number): void => {
    const tableElement = tableRef.value?.$el;
    if (!tableElement || !column.id) return;
    tableElement
      .querySelectorAll<HTMLTableColElement>(`colgroup > col[name="${column.id}"]`)
      .forEach((element) => element.setAttribute("width", String(width))); // Element Plus 的固定布局通过 col 宽度同时控制表头和单元格。
  };

  /**
   * 将当前会话中记住的列宽重新应用到 Element Plus 新建的列对象。
   */
  const restoreRuntimeWidths = (): void => {
    const columns = tableRef.value?.columns ?? [];
    let restored = false;
    columns.forEach((column) => {
      const width = runtimeWidths.get(getColumnKey(column));
      if (width == null) return;
      column.width = width;
      column.realWidth = width; // 列隐藏后重新出现时继续使用本次运行期间的宽度。
      restored = true;
    });
    if (restored) refreshTableLayout();
  };

  /**
   * 从事件坐标判断是否命中表头右侧的列宽拖拽热区。
   */
  const hitsResizeHandle = (clientX: number, header: HTMLElement): boolean => {
    const rect = header.getBoundingClientRect();
    return (
      rect.width > 12 &&
      clientX >= rect.right - RESIZE_HANDLE_WIDTH &&
      clientX <= rect.right + RESIZE_HANDLE_WIDTH
    );
  };

  /**
   * 仅在表头右侧八像素范围内把按下操作识别为列宽拖拽。
   */
  const isResizeHandle = (event: PointerEvent, header: HTMLElement): boolean => {
    return hitsResizeHandle(event.clientX, header);
  };

  /**
   * 在下一帧提交最新列宽并触发表格重新布局，避免高频指针事件重复渲染。
   */
  const scheduleWidthUpdate = (): void => {
    if (!session || session.frameId != null) return;
    session.frameId = requestAnimationFrame(() => {
      if (!session) return;
      session.frameId = null;
      session.column.width = session.pendingWidth;
      session.column.realWidth = session.pendingWidth; // 表头和单元格在拖动期间使用相同实时宽度。
      runtimeWidths.set(session.columnKey, session.pendingWidth); // 每一帧同步缓存，后续表格重绘可恢复。
      syncColumnElements(session.column, session.pendingWidth);
      refreshTableLayout();
    });
  };

  /**
   * 根据指针当前位置实时计算并提交目标列宽。
   */
  const handlePointerMove = (event: PointerEvent): void => {
    if (!session || event.pointerId !== session.pointerId) return;
    session.pendingWidth = Math.max(
      session.minWidth,
      Math.round(session.startWidth + event.clientX - session.startX)
    );
    scheduleWidthUpdate();
  };

  /**
   * 结束当前列宽拖拽并清理全局监听器。
   */
  const finishResize = (event?: PointerEvent): void => {
    if (!session || (event && event.pointerId !== session.pointerId)) return;
    const finished = session;
    if (finished.frameId != null) {
      cancelAnimationFrame(finished.frameId);
    }
    finished.column.width = finished.pendingWidth;
    finished.column.realWidth = finished.pendingWidth; // 松开时仅提交最后一帧，不再延迟改变视觉宽度。
    runtimeWidths.set(finished.columnKey, finished.pendingWidth); // 松开仅结束拖拽，宽度仍只保留在内存。
    syncColumnElements(finished.column, finished.pendingWidth);
    finished.header.classList.remove("realtime-resizing");
    armSuppressedClick(finished.header); // 指针释放后浏览器可能再派发一次手柄 click，需要阻止表头排序。
    session = null;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    document.removeEventListener("pointermove", handlePointerMove, true);
    document.removeEventListener("pointerup", finishResize, true);
    document.removeEventListener("pointercancel", finishResize, true);
    refreshTableLayout();
  };

  /**
   * 在捕获阶段接管 Element Plus 默认的鼠标松开后更新列宽行为。
   */
  const handlePointerDown = (event: PointerEvent): void => {
    if (event.button !== 0 || session) return;
    clearSuppressedClick(); // 新的按下开始前清理极少数未派发 click 留下的状态。
    const target = event.target instanceof Element ? event.target : null;
    const header = target?.closest("th") as HTMLElement | null;
    if (!header || !isResizeHandle(event, header)) return;

    const column = findColumn(header);
    if (!column) return;
    const property = column.property ?? "";
    const columnKey = getColumnKey(column);
    session = {
      pointerId: event.pointerId,
      column,
      columnKey,
      startX: event.clientX,
      startWidth: header.getBoundingClientRect().width,
      minWidth: getMinimumWidth(property),
      header,
      pendingWidth: header.getBoundingClientRect().width,
      frameId: null,
    };
    event.preventDefault();
    event.stopImmediatePropagation(); // 防止 Element Plus 同时启动只显示代理线的默认拖拽。
    header.classList.add("realtime-resizing");
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    document.addEventListener("pointermove", handlePointerMove, true);
    document.addEventListener("pointerup", finishResize, true);
    document.addEventListener("pointercancel", finishResize, true);
  };

  /**
   * 捕获由 PointerEvent 兼容鼠标触发的后续 mousedown，阻止 Element Plus 再启动默认代理线拖拽。
   */
  const handleMouseDown = (event: MouseEvent): void => {
    const target = event.target instanceof Element ? event.target : null;
    const header = target?.closest("th") as HTMLElement | null;
    if (!header || !hitsResizeHandle(event.clientX, header)) return;
    event.preventDefault();
    event.stopImmediatePropagation(); // 浏览器仍派发兼容鼠标事件时继续隔离组件默认拖拽逻辑。
  };

  /**
   * 处理由 PointerEvent 触发的兼容 click，避免拖动结束后误执行排序。
   */
  const handleClick = (event: MouseEvent): void => {
    if (!suppressNextClick || !suppressedClickHeader) return;
    const target = event.target instanceof Element ? event.target : null;
    const header = target?.closest("th");
    if (header !== suppressedClickHeader || !hitsResizeHandle(event.clientX, suppressedClickHeader)) {
      return; // 普通表头点击或其他列点击不受拖拽后的临时状态影响。
    }
    clearSuppressedClick();
    event.preventDefault();
    event.stopImmediatePropagation(); // 拖动手柄释放后的合成点击不应改变排序状态。
  };

  /**
   * 在表格渲染完成后绑定一次捕获阶段的指针监听器。
   */
  const bindRealtimeResize = async (): Promise<void> => {
    await nextTick();
    const tableElement = tableRef.value?.$el;
    if (!tableElement) return;
    restoreRuntimeWidths();
    if (tableElement.dataset.realtimeResize === "true") return;
    tableElement.dataset.realtimeResize = "true"; // 组件重复更新时避免重复注册监听器。
    tableElement.addEventListener("pointerdown", handlePointerDown, true);
    tableElement.addEventListener("mousedown", handleMouseDown, true); // 双重捕获兼容 WebView 的指针与鼠标事件派发顺序。
    tableElement.addEventListener("click", handleClick, true);
  };

  /**
   * 组件销毁时移除表格和文档级监听器。
   */
  const disposeRealtimeResize = (): void => {
    const tableElement = tableRef.value?.$el;
    tableElement?.removeEventListener("pointerdown", handlePointerDown, true);
    tableElement?.removeEventListener("mousedown", handleMouseDown, true);
    tableElement?.removeEventListener("click", handleClick, true);
    if (tableElement) {
      delete tableElement.dataset.realtimeResize;
    }
    finishResize();
    clearSuppressedClick();
  };

  onBeforeUnmount(disposeRealtimeResize);
  return { bindRealtimeResize };
};
