import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

/**
 * 共享的 headless Chromium + CDP 底座。
 *
 * `browser-smoke.ts` 与 `ui-self-audit.ts` 都要"起服务 → 起浏览器 → 连 CDP → 收干净"，
 * 这一套只应该有一份；重复一遍就会出现一边修了超时、另一边没修的情况。
 */

type PendingCall = {
  resolve: (value: unknown) => void;
  reject: (reason?: unknown) => void;
};

type SpawnedProcess = ReturnType<typeof Bun.spawn>;

export class CdpSession {
  private nextId = 1;
  private pending = new Map<number, PendingCall>();

  constructor(private readonly socket: WebSocket) {
    socket.addEventListener("message", (event) => {
      const message = JSON.parse(String(event.data));
      if (!message.id) return;
      const pending = this.pending.get(message.id);
      if (!pending) return;
      this.pending.delete(message.id);
      if (message.error) pending.reject(new Error(message.error.message));
      else pending.resolve(message.result);
    });
  }

  static async connect(wsUrl: string): Promise<CdpSession> {
    const socket = new WebSocket(wsUrl);
    await new Promise<void>((resolve, reject) => {
      socket.addEventListener("open", () => resolve(), { once: true });
      socket.addEventListener(
        "error",
        () => reject(new Error(`无法连接 DevTools: ${wsUrl}`)),
        { once: true },
      );
    });
    return new CdpSession(socket);
  }

  async send<T = unknown>(method: string, params: Record<string, unknown> = {}): Promise<T> {
    const id = this.nextId++;
    const promise = new Promise<T>((resolve, reject) => {
      this.pending.set(id, {
        resolve: (value) => resolve(value as T),
        reject,
      });
    });
    this.socket.send(JSON.stringify({ id, method, params }));
    return promise;
  }

  close(): void {
    this.socket.close();
  }
}

export interface HeadlessPageOptions {
  readonly port: number;
  readonly debugPort: number;
  readonly width: number;
  readonly height: number;
}

export interface HeadlessPage {
  readonly session: CdpSession;
  readonly appUrl: string;
  navigate(path: string): Promise<void>;
  evaluate(expression: string, awaitPromise?: boolean): Promise<unknown>;
  screenshot(): Promise<string>;
  resize(width: number, height: number): Promise<void>;
}

export async function withHeadlessPage<T>(
  options: HeadlessPageOptions,
  body: (page: HeadlessPage) => Promise<T>,
): Promise<T> {
  const appUrl = `http://127.0.0.1:${options.port}/`;
  const server = await ensureServer(appUrl, options.port);
  const userDataDir = await mkdtemp(join(tmpdir(), "yixian-ui-headless-"));
  const chrome = Bun.spawn([
    process.env.CHROME_PATH ?? findChrome(),
    "--headless=new",
    "--disable-gpu",
    "--no-first-run",
    "--no-default-browser-check",
    "--no-sandbox",
    "--remote-debugging-address=127.0.0.1",
    `--remote-debugging-port=${options.debugPort}`,
    `--user-data-dir=${userDataDir}`,
    "about:blank",
  ], { stderr: "pipe", stdout: "ignore" });

  let session: CdpSession | null = null;
  try {
    await waitForBrowserVersion(chrome, options.debugPort);
    const target = await createTarget(appUrl, options.debugPort);
    session = await CdpSession.connect(target.webSocketDebuggerUrl);
    await session.send("Runtime.enable");
    await session.send("Page.enable");
    const page = createPage(session, appUrl, options);
    await page.resize(options.width, options.height);
    await waitForReady(page);
    return await body(page);
  } finally {
    session?.close();
    chrome.kill();
    server?.kill();
    await rm(userDataDir, { recursive: true, force: true });
  }
}

function createPage(
  session: CdpSession,
  appUrl: string,
  options: HeadlessPageOptions,
): HeadlessPage {
  const page: HeadlessPage = {
    session,
    appUrl,
    async navigate(path: string): Promise<void> {
      await session.send("Page.navigate", { url: `${appUrl}${path}` });
      await waitForReady(page);
    },
    async evaluate(expression: string, awaitPromise = false): Promise<unknown> {
      return evaluateIn(session, expression, awaitPromise);
    },
    async screenshot(): Promise<string> {
      const result = await session.send<{ data: string }>("Page.captureScreenshot", {
        format: "png",
        captureBeyondViewport: false,
      });
      return result.data;
    },
    async resize(width: number, height: number): Promise<void> {
      await session.send("Emulation.setDeviceMetricsOverride", {
        width,
        height,
        deviceScaleFactor: 1,
        mobile: false,
      });
    },
  };
  void options;
  return page;
}

export async function evaluateIn(
  session: CdpSession,
  expression: string,
  awaitPromise = false,
): Promise<unknown> {
  const result = await session.send<{
    exceptionDetails?: { text?: string };
    result: { value?: unknown; description?: string };
  }>("Runtime.evaluate", { expression, awaitPromise, returnByValue: true });
  if (result.exceptionDetails) {
    throw new Error(
      result.exceptionDetails.text ?? result.result.description ?? "Runtime.evaluate failed",
    );
  }
  return result.result.value;
}

export async function waitForReady(page: HeadlessPage): Promise<void> {
  for (let index = 0; index < 100; index += 1) {
    const ready = await page.evaluate(
      "document.readyState === 'complete' && !!document.querySelector('#app')",
    );
    if (ready === true) return;
    await Bun.sleep(100);
  }
  throw new Error("UI 页面未就绪");
}

async function ensureServer(appUrl: string, port: number): Promise<SpawnedProcess | null> {
  if (await isAppReady(appUrl)) return null;
  const server = Bun.spawn(["python3", "-m", "http.server", String(port)], {
    stdout: "ignore",
    stderr: "ignore",
  });
  const deadline = Date.now() + 5_000;
  while (Date.now() < deadline) {
    if (await isAppReady(appUrl)) return server;
    await Bun.sleep(100);
  }
  server.kill();
  throw new Error(`无法启动 UI 服务: ${appUrl}`);
}

async function isAppReady(appUrl: string): Promise<boolean> {
  try {
    const response = await fetch(appUrl, { cache: "no-store" });
    return response.ok;
  } catch {
    return false;
  }
}

async function waitForBrowserVersion(chrome: SpawnedProcess, debugPort: number): Promise<void> {
  const deadline = Date.now() + 10_000;
  while (Date.now() < deadline) {
    if (chrome.exitCode !== null) break;
    try {
      const response = await fetch(`http://127.0.0.1:${debugPort}/json/version`);
      if (response.ok) return;
    } catch {
      // Browser is still starting.
    }
    await Bun.sleep(100);
  }
  throw new Error(`Chromium DevTools 未就绪: 127.0.0.1:${debugPort}`);
}

async function createTarget(
  url: string,
  debugPort: number,
): Promise<{ webSocketDebuggerUrl: string }> {
  const response = await fetch(
    `http://127.0.0.1:${debugPort}/json/new?${encodeURIComponent(url)}`,
    { method: "PUT" },
  );
  if (!response.ok) throw new Error(`创建 Chrome target 失败: ${response.status}`);
  return await response.json();
}

export function findChrome(): string {
  const candidates = [
    "/usr/bin/chromium",
    "/usr/bin/chromium-browser",
    "/usr/bin/google-chrome",
    "/usr/bin/google-chrome-stable",
  ];
  for (const candidate of candidates) {
    try {
      if (Bun.spawnSync(["test", "-x", candidate]).exitCode === 0) return candidate;
    } catch {
      // Try the next candidate.
    }
  }
  throw new Error("未找到 Chromium/Chrome，可用 CHROME_PATH 指定");
}
