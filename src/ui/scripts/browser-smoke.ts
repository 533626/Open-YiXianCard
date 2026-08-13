import { withHeadlessPage } from "./lib/headless-page";
import type { HeadlessPage } from "./lib/headless-page";

const PORT = Number(process.env.UI_SMOKE_PORT ?? 3001);
const DEBUG_PORT = Number(process.env.UI_SMOKE_DEBUG_PORT ?? 9223);

async function main(): Promise<void> {
  await withHeadlessPage(
    { port: PORT, debugPort: DEBUG_PORT, width: 1280, height: 800 },
    async (page) => {
      await runSetupSmoke(page);
      await page.resize(1600, 1000);
      await runWorkerBattleSmoke(page);
    },
  );
  console.log(
    "UI smoke passed: setup interactions, Rust/WASM Worker battle, canonical hook chain, live frame navigation, right-column module switching, and solver result",
  );
}






async function runSetupSmoke(page: HeadlessPage): Promise<void> {
  const result = await page.evaluate(`(${smokeInPage.toString()})()`, true);
  if (!result || typeof result !== "object") throw new Error("UI smoke 没有返回结果");
  const smoke = result as { ok?: boolean; error?: string };
  if (!smoke.ok) throw new Error(smoke.error ?? "UI smoke failed");
}

async function runWorkerBattleSmoke(page: HeadlessPage): Promise<void> {
  const result = await page.evaluate(`(${workerBattleSmokeInPage.toString()})()`, true);
  if (!result || typeof result !== "object") throw new Error("Worker battle smoke 没有返回结果");
  const smoke = result as { ok?: boolean; error?: string };
  if (!smoke.ok) throw new Error(smoke.error ?? "Worker battle smoke failed");
}


function smokeInPage(): Promise<{ ok: true } | { ok: false; error: string }> {
  const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
  const fail = (message: string) => ({ ok: false as const, error: message });
  const click = (selector: string) => {
    const element = document.querySelector<HTMLElement>(selector);
    if (!element) throw new Error(`缺少元素: ${selector}`);
    element.click();
  };
  const pickTalent = async (talentId: number) => {
    click(".player-panel[data-side='p1'] button[data-action='select-talent-slot'][data-slot='4']");
    await sleep(40);
    click(`[data-action='pick-talent'][data-talent-id='${talentId}']`);
    await sleep(40);
    click(".picker-popup-close");
    await sleep(40);
  };
  const panelText = () => document.querySelector<HTMLElement>(".player-panel[data-side='p1']")?.innerText ?? "";
  const placeCard = async (side: string) => {
    click(`.player-panel[data-side='${side}'] button[data-action='select-slot'][data-slot='0']`);
    await sleep(40);
    click("[data-action='pick-card']:not(:disabled)");
    await sleep(40);
    document.querySelector<HTMLElement>(".picker-popup-close")?.click();
    await sleep(40);
  };

  return (async () => {
    try {
      // 覆盖机制已删除：左列构筑任何时候都可直接操作，右列才承载功能面板。
      if (document.querySelector(".workflow-nav")) return fail("工作流入口应已删除");
      if (document.querySelector("[data-action='toggle-workflow-panel']")) {
        return fail("全高覆盖控件应已删除");
      }
      const buildRect = document.querySelector<HTMLElement>("#free-build")
        ?.getBoundingClientRect();
      if (!buildRect || buildRect.height < 200) {
        return fail(`构筑列尺寸异常: ${buildRect?.width ?? 0}x${buildRect?.height ?? 0}`);
      }
      if (document.documentElement.scrollWidth > window.innerWidth) return fail("页面出现横向滚动");
      if (!document.querySelector('.combined-battle [data-module="advice"]')) {
        return fail("获胜建议不在右列");
      }

      click("[data-action='reset']");
      await sleep(60);
      const initialCharacter = document.querySelector<HTMLElement>(
        ".player-panel[data-side='p1'] [data-action='open-character-picker']",
      );
      if (initialCharacter?.innerText.trim() !== "选择角色") return fail("初始角色不为空");
      initialCharacter.click();
      await sleep(40);
      click("[data-action='pick-character'][data-character-id='4000004']");
      await sleep(40);
      click(".picker-popup-close");
      await sleep(40);
      click(".player-panel[data-side='p1'] button[data-action='select-slot'][data-slot='0']");
      await sleep(40);
      const library = document.querySelector<HTMLElement>("[data-card-picker-scroll]");
      if (!library) return fail("连续选卡卡池未出现");
      library.scrollTop = Math.min(120, Math.max(0, library.scrollHeight - library.clientHeight));
      const pickerScrollTop = library.scrollTop;
      click("[data-action='pick-card']:not(:disabled)");
      await sleep(40);
      if (!document.querySelector(".card-popup")) return fail("选卡后工作区意外关闭");
      if (!document.querySelector(".player-panel[data-side='p1'] .deck-slot[data-slot='1'].editing")) {
        return fail("选卡后未自动前进到下一槽");
      }
      const nextLibrary = document.querySelector<HTMLElement>("[data-card-picker-scroll]");
      if (nextLibrary?.scrollTop !== pickerScrollTop) {
        return fail(`选卡后卡池滚动位置漂移: ${pickerScrollTop} -> ${nextLibrary?.scrollTop ?? -1}`);
      }
      // 顶部八槽已移除：直接点左侧构筑区卡槽切换目标，右键清空后浮层收回。
      click(".player-panel[data-side='p1'] button[data-action='select-slot'][data-slot='0']");
      await sleep(40);
      const firstPickerSlot = document.querySelector<HTMLElement>(
        ".player-panel[data-side='p1'] .card-face[data-action='select-slot'][data-slot='0']",
      );
      firstPickerSlot?.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true }));
      await sleep(40);
      if (!document.querySelector(".player-panel[data-side='p1'] .deck-slot[data-slot='0'].empty")) {
        return fail("右键未清空卡槽");
      }
      if (document.querySelector(".card-popup")) return fail("右键清槽后选卡浮层应收回");
      click(".player-panel[data-side='p1'] button[data-action='select-slot'][data-slot='0']");
      await sleep(40);
      const search = document.querySelector<HTMLInputElement>("#cardSearch");
      if (!search) return fail("选卡搜索框未出现");
      search.focus();
      for (const char of ["极", "迎"]) {
        search.value += char;
        search.dispatchEvent(new InputEvent("input", { bubbles: true, data: char, inputType: "insertText" }));
        await sleep(10);
      }
      const nextSearch = document.querySelector<HTMLInputElement>("#cardSearch");
      if (nextSearch?.value !== "极迎") return fail(`搜索值异常: ${nextSearch?.value ?? "<missing>"}`);
      if (document.activeElement?.id !== "cardSearch") return fail("搜索输入后焦点丢失");
      if (!document.body.innerText.includes("极•迎风掌")) return fail("搜索结果未显示极•迎风掌");
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
      await sleep(40);
      if (document.querySelector(".card-popup")) return fail("搜索框聚焦时 Esc 未退出选卡");

      await pickTalent(30_001);
      const duanTiText = panelText();
      if (!duanTiText.includes("上限\n197")) return fail(`锻体未刷新上限: ${duanTiText}`);

      await pickTalent(30_146);
      const text = panelText();
      const physique = document.querySelector<HTMLInputElement>(
        ".player-panel[data-side='p1'] input[data-buff='physique']",
      )?.value;
      if (!text.includes("上限\n193")) return fail("体修入道未刷新生命上限");
      if (physique !== "88" || !text.includes("/97")) return fail(`体修入道体魄异常: ${physique}, ${text}`);
      // 就绪闸：双方各 ≥1 张场上牌才自动推演。给 p2 选角色后，再给双方各摆一张牌触发右侧实时战斗。
      click(".player-panel[data-side='p2'] [data-action='open-character-picker']");
      await sleep(40);
      click("[data-action='pick-character'][data-character-id='4000005']");
      await sleep(40);
      document.querySelector<HTMLElement>(".picker-popup-close")?.click();
      await sleep(40);
      await placeCard("p1");
      await placeCard("p2");
      return { ok: true as const };
    } catch (error) {
      return fail(error instanceof Error ? error.message : String(error));
    }
  })();
}

function workerBattleSmokeInPage(): Promise<{ ok: true } | { ok: false; error: string }> {
  const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
  const fail = (message: string) => ({ ok: false as const, error: message });

  return (async () => {
    try {
      for (let index = 0; index < 200; index += 1) {
        const error = document.querySelector<HTMLElement>(".error-bar")?.innerText.trim();
        if (error) return fail(`Worker 战斗失败: ${error}`);
        if (document.querySelector(".battle-view")) break;
        await sleep(50);
      }
      if (!document.querySelector(".battle-view")) return fail("Worker 战斗未在 10 秒内完成");
      // 宽屏下引擎透视固定在右列；生命曲线与获胜建议只替换左下 companion。
      const bodies = [...document.querySelectorAll<HTMLElement>(".battle-module-body")];
      if (bodies.length !== 1) return fail(`右列同时挂着 ${bodies.length} 个模块体`);
      if (bodies[0]!.dataset.module !== "insight") {
        return fail(`固定引擎透视显示的是 ${bodies[0]!.dataset.module}`);
      }
      const insight = bodies[0]!;
      // 钩子链要真的从 Rust/WASM 穿过 Worker 到页面上，不能只是布局没报错。
      const hookSteps = insight.querySelectorAll(".hook-step").length;
      const hookChanges = insight.querySelectorAll(".hook-change").length;
      if (hookSteps === 0 || hookChanges === 0) {
        return fail(
          `引擎透视没有渲染钩子链: steps=${hookSteps} changes=${hookChanges}`,
        );
      }
      const setupRect = document.querySelector<HTMLElement>("#free-build")?.getBoundingClientRect();
      if (!setupRect || setupRect.width < 200) return fail("展开右列面板后左列构筑被压掉");
      const insightList = insight.querySelector<HTMLElement>(".engine-step-list");
      if (!insightList || insightList.scrollWidth > insightList.clientWidth) {
        return fail(
          `引擎日志出现横向滚动: ${insightList?.clientWidth ?? 0}/${insightList?.scrollWidth ?? 0}`,
        );
      }
      const labels = [...document.querySelectorAll<HTMLElement>(".insight-companion .flow-label")];
      if (labels.map((label) => label.innerText.trim()).join("|") !== "玩家一生命|玩家二生命|生命差") {
        return fail(`生命曲线标签不完整: ${labels.map((label) => label.innerText).join("|")}`);
      }
      if (labels.some((label) => label.scrollWidth > label.clientWidth + 1)) {
        return fail("生命曲线标签被折叠");
      }
      const rect = (element: Element | null) => {
        const value = element?.getBoundingClientRect();
        return value
          ? { x: value.x, y: value.y, width: value.width, height: value.height }
          : null;
      };
      const beforeSwitch = {
        insight: rect(insight),
        companion: rect(document.querySelector(".insight-companion")),
      };

      // 时间轴与帧导航不属于任何模块，所以在引擎透视下就能直接逐动翻。
      if (!document.querySelector(".battle-progress-rail .battle-progress-dot")) {
        return fail("时间轴不在模块外的公共区域");
      }
      const dots = document.querySelectorAll<HTMLButtonElement>(
        ".battle-progress-rail .battle-progress-dot[data-action='jump-frame']",
      );
      if (dots.length < 2) return fail("没有可逐动跳转的时间轴点");
      const beforeFrame = document.querySelector<HTMLElement>(
        ".battle-progress-dot.selected",
      )?.dataset.frame;
      // 战斗结束后默认落在第一个回合结束帧，挑一个帧号不同的点跳过去验证轨道能推进。
      const target = [...dots].find((dot) => dot.dataset.frame !== beforeFrame);
      if (!target) return fail("时间轴点帧号与当前帧完全相同，无法验证推进");
      target.click();
      await sleep(60);
      const afterFrame = document.querySelector<HTMLElement>(
        ".battle-progress-dot.selected",
      )?.dataset.frame;
      if (!afterFrame || afterFrame === beforeFrame) {
        return fail("回放导航没有推进当前帧");
      }

      const adviceTab = document.querySelector<HTMLButtonElement>(
        '.battle-module-tab[data-module="advice"]',
      );
      if (!adviceTab) return fail("右列缺少获胜建议选项卡");
      adviceTab.click();
      await sleep(80);
      if (!document.querySelector('.insight-companion[data-module="advice"]')) {
        return fail("获胜建议未进入左下 companion");
      }
      const solverPanelBeforeRun = document.querySelector<HTMLElement>(".solver-panel");
      const solverTasks = document.querySelector<HTMLElement>(".solver-task-tabs");
      if (!solverPanelBeforeRun || !solverTasks) return fail("求解工作区缺少任务栏");
      if (document.querySelector("[data-action='toggle-solver']")) {
        return fail("求解工作区仍有多余折叠入口");
      }
      if (
        Math.abs(
          solverPanelBeforeRun.getBoundingClientRect().width -
          solverTasks.getBoundingClientRect().width
        ) > 1
      ) {
        return fail("求解任务栏没有横向撑满工作区");
      }
      if (getComputedStyle(solverPanelBeforeRun).overflowY !== "visible") {
        return fail("求解面板仍是内层滚动容器");
      }
      document.querySelector<HTMLButtonElement>("[data-solver-task='hand']")?.click();
      await sleep(30);
      if (!document.querySelector(".solver-hand-input")?.textContent?.includes("当前手牌")) {
        return fail("手牌求解没有显示当前手牌");
      }
      const poolTask = document.querySelector<HTMLButtonElement>("[data-solver-task='pool']");
      poolTask?.click();
      await sleep(30);
      const poolExhaustive = document.querySelector<HTMLButtonElement>(
        "[data-solver-method='exhaustive']",
      );
      if (!poolExhaustive?.disabled || !poolExhaustive.classList.contains("unavailable")) {
        return fail("卡池求解仍可选择穷举");
      }
      document.querySelector<HTMLButtonElement>("[data-solver-task='order']")?.click();
      await sleep(30);
      const afterSwitch = {
        insight: rect(document.querySelector(".battle-module-body")),
        companion: rect(document.querySelector(".insight-companion")),
      };
      const shifted = (before: typeof beforeSwitch.insight, after: typeof afterSwitch.insight) =>
        !before || !after ||
        Math.abs(before.x - after.x) > 1 ||
        Math.abs(before.y - after.y) > 1 ||
        Math.abs(before.width - after.width) > 1 ||
        Math.abs(before.height - after.height) > 1;
      if (
        shifted(beforeSwitch.insight, afterSwitch.insight) ||
        shifted(beforeSwitch.companion, afterSwitch.companion)
      ) {
        return fail("生命曲线/获胜建议切换引发分栏 CLS");
      }
      // 求解在这套小卡组上瞬时返回，取消窗口观测不到；这里只验证"点了能出结果"。
      // 运行态与取消态的渲染由 layout-verification 的注入式契约确定性覆盖，
      // 不在浏览器里赌时序。
      const solve = document.querySelector<HTMLButtonElement>("[data-action='solve-active']");
      if (!solve) return fail("获胜建议模块缺少求解按钮");
      solve.click();
      let solverPanel: HTMLElement | null = null;
      for (let attempt = 0; attempt < 200; attempt += 1) {
        await sleep(50);
        solverPanel = document.querySelector<HTMLElement>(".solver-panel");
        if (solverPanel?.classList.contains("has-result")) break;
      }
      if (!solverPanel?.classList.contains("has-result")) {
        return fail(`求解未产出结果: panel=${solverPanel?.className ?? "<none>"}`);
      }
      if (document.querySelector("[data-action='cancel-solver']")) return fail("取消后仍显示取消按钮");
      if (!document.querySelector(".solver-baseline") || document.querySelector(".winning-recommendation")) {
        return fail("求解结果没有使用 TUI 式基准线，或仍保留上方推荐框");
      }
      const solverRows = [...document.querySelectorAll<HTMLElement>(".solver-row")];
      if (solverRows.length === 0 || !solverRows.every((row) =>
        row.querySelector(".solver-order-digits") &&
        row.scrollWidth <= row.clientWidth &&
        row.getBoundingClientRect().height <= 26
      )) {
        return fail("候选牌序未保持 TUI 式单行数字布局");
      }
      return { ok: true as const };
    } catch (error) {
      return fail(error instanceof Error ? error.message : String(error));
    }
  })();
}


void main();
