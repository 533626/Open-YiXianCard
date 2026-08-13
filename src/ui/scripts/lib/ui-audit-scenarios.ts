/**
 * 场景定义与页内检查规则。
 *
 * `prepare` 和 `collectFindingsInPage` 会被 `toString()` 注入页面执行，所以必须自包含：
 * 不能引用模块作用域的任何东西。
 */

export interface AuditFinding {
  readonly scenario: string;
  readonly rule: string;
  /** 稳定定位串；进基线台账的 id 由 scenario/rule/key 三段拼成，不能带随机数或时间。 */
  readonly key: string;
  readonly detail: string;
}

export interface AuditScenario {
  readonly name: string;
  readonly path: string;
  /** 切到这个模块选项卡；省略表示用默认选中的模块。 */
  readonly module?: string;
}

const BATTLE_PATH = "index.html";

export const AUDIT_SCENARIOS: readonly AuditScenario[] = [
  { name: "setup-empty", path: "index.html" },
  { name: "battle-verdict", path: BATTLE_PATH },
  { name: "battle-trajectory", path: BATTLE_PATH, module: "trajectory" },
  { name: "battle-advice", path: BATTLE_PATH, module: "advice" },
];

/**
 * 宽屏右侧固定引擎透视，生命曲线/获胜建议在 companion 槽位互斥。
 *
 * 这个函数会被注入页面执行，必须自包含 —— 不能引用模块作用域，也不能闭包捕获参数，
 * 目标模块只能由调用方以字面量传进来。
 */
export async function prepareScenarioInPage(
  module: string | null,
): Promise<{ ok: true } | { ok: false; error: string }> {
  if (module !== null && !document.querySelector(".battle-view")) {
    const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
    const waitFor = async (selector: string): Promise<HTMLElement | null> => {
      for (let index = 0; index < 200; index += 1) {
        const element = document.querySelector<HTMLElement>(selector);
        if (element) return element;
        await sleep(50);
      }
      return null;
    };
    const clickFirstCard = async (side: string): Promise<boolean> => {
      const slot = await waitFor(`.player-panel[data-side="${side}"] button[data-action="select-slot"][data-slot="0"]`);
      if (!slot) return false;
      slot.click();
      const card = await waitFor("[data-action=pick-card]:not(:disabled)");
      if (!card) return false;
      card.click();
      document.querySelector<HTMLElement>(".picker-popup-close")?.click();
      return true;
    };
    const chooseCharacter = async (side: string, characterId: string): Promise<boolean> => {
      const trigger = await waitFor(`.player-panel[data-side="${side}"] [data-action="open-character-picker"]`);
      if (!trigger) return false;
      trigger.click();
      const character = await waitFor(`[data-action="pick-character"][data-character-id="${characterId}"]`);
      if (!character) return false;
      character.click();
      document.querySelector<HTMLElement>(".picker-popup-close")?.click();
      return true;
    };
    if (!await chooseCharacter("p1", "4000005") ||
        !await chooseCharacter("p2", "4000005") ||
        !await clickFirstCard("p1") ||
        !await clickFirstCard("p2")) {
      return { ok: false as const, error: "无 fixture 公共审计场景无法构造最小战斗" };
    }
    if (!await waitFor(".battle-view")) {
      return { ok: false as const, error: "无 fixture 公共审计场景战斗未完成" };
    }
  }
  for (let index = 0; index < 200; index += 1) {
    if (module === null || document.querySelector(".battle-verdict")) break;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  if (module === null) {
    return document.querySelector("#app")
      ? { ok: true as const }
      : { ok: false as const, error: "页面没有渲染出 #app" };
  }
  // 曲线模块的选项卡是 生命/伤害 分段开关，没有单独的 data-module 按钮；
  // 点生命档即切入曲线模块。
  const tab = document.querySelector<HTMLButtonElement>(
    module === "trajectory"
      ? '.trajectory-option[data-metric="life"]'
      : `.battle-module-tab[data-module="${module}"]`,
  );
  if (!tab) return { ok: false as const, error: `没有模块选项卡 ${module}` };
  tab.click();
  await new Promise((resolve) => setTimeout(resolve, 200));
  const target = document.querySelector<HTMLElement>(
    module === "insight" ? ".battle-module-body" : ".insight-companion",
  );
  return target?.dataset.module === module
    ? { ok: true as const }
    : { ok: false as const, error: `点了 ${module} 之后显示的还是 ${target?.dataset.module}` };
}

export function collectFindingsInPage(scenario: string): AuditFinding[] {
  const REGION_SELECTORS = [
    ".battle-verdict",
    ".battle-module-body",
    ".insight-companion",
    ".battle-module-tabs",
    ".player-panel",
    ".solver-panel",
    ".deck-diagnostic-panel",
    ".result-empty",
  ];
  const findings: AuditFinding[] = [];
  const push = (rule: string, key: string, detail: string): void => {
    findings.push({ scenario, rule, key, detail });
  };
  const viewportHeight = window.innerHeight;

  if (document.documentElement.scrollWidth > window.innerWidth + 1) {
    push(
      "page-overflow-x",
      "document",
      `横向溢出 ${document.documentElement.scrollWidth - window.innerWidth}px`,
    );
  }

  const regions = REGION_SELECTORS.flatMap((selector) =>
    [...document.querySelectorAll<HTMLElement>(selector)].map((element, index) => ({
      element,
      key: countOf(selector) > 1 ? `${selector}[${index}]` : selector,
    }))
  );

  for (const region of regions) {
    const rect = region.element.getBoundingClientRect();
    if (rect.height === 0) continue;
    // 目标形态是"不出滚动条、轮换占用下方"，所以任何越过视口下沿的区块都算问题，
    // 不管页面能不能滚到它。
    if (rect.bottom > viewportHeight + 1) {
      push(
        "below-fold",
        region.key,
        `下沿超出视口 ${Math.round(rect.bottom - viewportHeight)}px`,
      );
    }
    auditRegionText(region.element, region.key);
  }

  auditTimelineGrouping();
  auditModuleFill();

  return findings;

  /**
   * 选中的模块必须占满右列剩余高度，内容也必须跟着铺开。矮一截就说明布局没接上，
   * 而不是内容少 —— 用户报过"右下太矮"就是这个。
   */
  function auditModuleFill(): void {
    const column = document.querySelector<HTMLElement>(".combined-battle");
    const body = document.querySelector<HTMLElement>(".battle-module-body:not(.standalone)");
    if (!column || !body) return;
    const columnRect = column.getBoundingClientRect();
    const bodyRect = body.getBoundingClientRect();
    if (bodyRect.height === 0) return;
    const unused = Math.round(columnRect.bottom - bodyRect.bottom);
    if (unused > 12) {
      push(
        "module-fill",
        body.dataset.module ?? "unknown",
        `模块下沿离右列底部还差 ${unused}px`,
      );
    }
    const content = body.firstElementChild as HTMLElement | null;
    if (!content) return;
    const contentRect = content.getBoundingClientRect();
    const slack = Math.round(bodyRect.height - contentRect.height);
    if (slack > 12) {
      push(
        "module-fill",
        `${body.dataset.module ?? "unknown"} 内容`,
        `模块高 ${Math.round(bodyRect.height)}px，内容只用了 ${Math.round(contentRect.height)}px`,
      );
    }
  }

  /**
   * 时间轴的一个点必须正好对应一方的一次完整行动（含再动追加的出牌），所以
   * 点与 actorTurn 一一对应：两个点落在同一个 actorTurn 就是把一次行动切碎了，
   * 引擎透视里有而时间轴上没有的 actorTurn 则是漏掉了一次行动。
   */
  function auditTimelineGrouping(): void {
    const dots = [...document.querySelectorAll<HTMLElement>(".battle-progress-dot")];
    if (dots.length === 0) return;
    const seen = new Map<string, number>();
    for (const dot of dots) {
      const actorTurn = dot.dataset.actorTurn;
      if (actorTurn === undefined || !/^\d+$/u.test(actorTurn)) {
        push("timeline-grouping", `dot ${dot.textContent?.trim() ?? "?"}`, "点没有标出 actorTurn");
        continue;
      }
      seen.set(actorTurn, (seen.get(actorTurn) ?? 0) + 1);
    }
    for (const [actorTurn, count] of seen) {
      if (count > 1) {
        push(
          "timeline-grouping",
          `actorTurn ${actorTurn}`,
          `同一 actorTurn 被切成 ${count} 个点`,
        );
      }
    }
    const track = document.querySelector<HTMLElement>(".battle-progress-track");
    const rail = track?.parentElement;
    if (track && rail) {
      const trackWidth = track.getBoundingClientRect().width;
      const railStyle = window.getComputedStyle(rail);
      const railWidth = rail.getBoundingClientRect().width -
        Number.parseFloat(railStyle.paddingLeft) - Number.parseFloat(railStyle.paddingRight);
      if (railWidth > 0 && trackWidth < railWidth - 8) {
        push(
          "timeline-grouping",
          "track",
          `时间轴只占了所在行的 ${Math.round((trackWidth / railWidth) * 100)}%，左右空了出来`,
        );
      }
    }
  }

  function auditRegionText(region: HTMLElement, regionKey: string): void {
    const repeats = new Map<string, number>();
    for (const element of region.querySelectorAll<HTMLElement>("*")) {
      if (element.closest("[data-audit-ignore]")) continue;
      if (!isVisible(element)) continue;
      if (hasElementChildren(element)) continue;
      const text = (element.textContent ?? "").trim();
      if (text.length === 0) continue;

      if (element.scrollWidth > element.clientWidth + 1 && element.clientWidth > 0) {
        push(
          "text-clipped",
          `${regionKey} ${labelFor(element, text)}`,
          `"${text}" 被裁掉 ${element.scrollWidth - element.clientWidth}px`,
        );
      }

      if (isBareNumber(text) && !hasExplanation(element)) {
        push(
          "bare-number",
          `${regionKey} ${labelFor(element, text)}`,
          `数字 "${text}" 没有单位也没有 title/aria-label 解释`,
        );
      }

      if (text.length >= 2 && text.length <= 12 && !isInRepeatedCollection(element)) {
        repeats.set(text, (repeats.get(text) ?? 0) + 1);
      }
    }
    for (const [text, count] of repeats) {
      // 两次通常是 p1/p2 成对，三次以上才是同一信息在一个区块里重复呈现。
      if (count < 3) continue;
      push("repeated-text", `${regionKey} "${text}"`, `同一区块内出现 ${count} 次`);
    }
  }

  /**
   * 元素处在一个同构集合里（8 个卡槽、3 行转折点）时，同一段文案重复出现是
   * 每一项自己的信息，不是同一条信息被讲了多遍。
   */
  function isInRepeatedCollection(element: HTMLElement): boolean {
    let node: HTMLElement | null = element.parentElement;
    for (let depth = 0; node && depth < 4; depth += 1) {
      const parent = node.parentElement;
      if (parent) {
        const sameShape = [...parent.children].filter(
          (sibling) => sibling !== node && sibling.className === node!.className,
        );
        if (sameShape.length >= 2) return true;
      }
      node = parent;
    }
    return false;
  }

  function countOf(selector: string): number {
    return document.querySelectorAll(selector).length;
  }

  function isVisible(element: HTMLElement): boolean {
    const rect = element.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return false;
    const style = window.getComputedStyle(element);
    return style.visibility !== "hidden" && style.display !== "none";
  }

  function hasElementChildren(element: HTMLElement): boolean {
    return element.children.length > 0;
  }

  function isBareNumber(text: string): boolean {
    return /^[+-]?\d+(?:\.\d+)?$/.test(text);
  }

  function hasExplanation(element: HTMLElement): boolean {
    if (element.title.trim().length > 0) return true;
    if ((element.getAttribute("aria-label") ?? "").trim().length > 0) return true;
    const container = element.parentElement;
    if (!container) return false;
    if ((container.getAttribute("aria-label") ?? "").trim().length > 0) return true;
    if (container.title.trim().length > 0) return true;
    // 兄弟节点里有文字标签也算说明，比如「动数 27」。
    const siblingText = (container.textContent ?? "").replace(element.textContent ?? "", "");
    return /[\p{Script=Han}A-Za-z%]/u.test(siblingText);
  }

  function labelFor(element: HTMLElement, text: string): string {
    const classes = element.className
      .split(/\s+/u)
      .filter((name) => name.length > 0 && !/selected|active|open/u.test(name))
      .slice(0, 2)
      .join(".");
    const tag = element.tagName.toLowerCase();
    return classes.length > 0 ? `${tag}.${classes}` : `${tag}:${text.slice(0, 8)}`;
  }
}
