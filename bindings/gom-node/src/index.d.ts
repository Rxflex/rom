export interface CookieEntry {
  name: string;
  value: string;
  domain?: string;
  hostOnly?: boolean;
  path?: string;
  secure?: boolean;
  httpOnly?: boolean;
  sameSite?: "Lax" | "Strict" | "None";
  expiresAt?: number | null;
}

export type CookieInput =
  | string
  | CookieEntry[]
  | Record<string, string | number | boolean | null | undefined>;

export type StorageInput =
  | string
  | Record<string, string | number | boolean | null | undefined>
  | Array<[string, string | number | boolean | null | undefined]>;

export interface RuntimeConfig {
  href?: string;
  referrer?: string;
  user_agent?: string;
  app_name?: string;
  platform?: string;
  language?: string;
  languages?: string[];
  hardware_concurrency?: number;
  device_memory?: number;
  webdriver?: boolean;
  cors_enabled?: boolean;
  proxy_url?: string | null;
  cookie_store?: CookieInput | null;
  cookies?: CookieInput | null;
  local_storage?: StorageInput | null;
  session_storage?: StorageInput | null;
  localStorage?: StorageInput | null;
  sessionStorage?: StorageInput | null;
}

export interface WaitForSelectorOptions {
  state?: "attached" | "detached" | "visible" | "hidden";
  timeout?: number;
  polling?: number;
}

export interface WaitForFunctionOptions {
  timeout?: number;
  polling?: number | "raf";
}

export interface NavigateOptions {
  method?: string;
  headers?: Record<string, string | number | boolean>;
  body?: string | null;
  timeout?: number;
  waitUntil?: "commit" | "domcontentloaded" | "load" | "networkidle";
}

export interface NavigateResult {
  url: string;
  status: number;
  ok: boolean;
  redirected: boolean;
  contentType: string | null;
  bodyLength: number;
}

export declare class RomLocator {
  click(options?: WaitForSelectorOptions): Promise<void>;
  fill(value: string | number | boolean, options?: WaitForSelectorOptions): Promise<void>;
  textContent(options?: WaitForSelectorOptions): Promise<string | null>;
  innerHTML(options?: WaitForSelectorOptions): Promise<string | null>;
  waitFor(options?: WaitForSelectorOptions): Promise<RomLocator | null>;
  evaluate<T = unknown>(pageFunction: string | ((element: unknown, arg?: unknown) => T), arg?: unknown): Promise<T>;
}

export declare class RomPage {
  locator(selector: string): RomLocator;
  evaluate<T = unknown>(pageFunction: string | ((arg?: unknown) => T), arg?: unknown): Promise<T>;
  content(): Promise<string>;
  setContent(html: string, options?: NavigateOptions): Promise<void>;
  goto(url: string, options?: NavigateOptions): Promise<NavigateResult>;
  waitForSelector(selector: string, options?: WaitForSelectorOptions): Promise<RomLocator | null>;
  waitForFunction<T = unknown>(
    pageFunction: string | ((arg?: unknown) => T),
    arg?: unknown,
    options?: WaitForFunctionOptions,
  ): Promise<T>;
  click(selector: string, options?: WaitForSelectorOptions): Promise<void>;
  fill(selector: string, value: string | number | boolean, options?: WaitForSelectorOptions): Promise<void>;
  textContent(selector: string, options?: WaitForSelectorOptions): Promise<string | null>;
  innerHTML(selector: string, options?: WaitForSelectorOptions): Promise<string | null>;
}

export declare class RomRuntime {
  constructor(config?: RuntimeConfig);
  eval(script: string): Promise<string>;
  evalAsync(script: string): Promise<string>;
  evalJson<T = unknown>(script: string, options?: { async?: boolean }): Promise<T>;
  surfaceSnapshot(): Promise<unknown>;
  fingerprintProbe(): Promise<unknown>;
  runFingerprintJsHarness(): Promise<unknown>;
  fingerprintJsVersion(): Promise<string>;
  hasLiveSession(): boolean;
  page(): RomPage;
  goto(url: string, options?: NavigateOptions): Promise<NavigateResult>;
  setContent(html: string, options?: NavigateOptions): Promise<void>;
  content(): Promise<string>;
  evaluate<T = unknown>(pageFunction: string | ((arg?: unknown) => T), arg?: unknown): Promise<T>;
  waitForSelector(selector: string, options?: WaitForSelectorOptions): Promise<RomLocator | null>;
  waitForFunction<T = unknown>(
    pageFunction: string | ((arg?: unknown) => T),
    arg?: unknown,
    options?: WaitForFunctionOptions,
  ): Promise<T>;
  click(selector: string, options?: WaitForSelectorOptions): Promise<void>;
  fill(selector: string, value: string | number | boolean, options?: WaitForSelectorOptions): Promise<void>;
  textContent(selector: string, options?: WaitForSelectorOptions): Promise<string | null>;
  innerHTML(selector: string, options?: WaitForSelectorOptions): Promise<string | null>;
  locator(selector: string): RomLocator;
}

export declare function createRuntime(config?: RuntimeConfig): RomRuntime;
export declare function hasNativeBinding(): boolean;
