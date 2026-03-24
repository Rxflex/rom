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
}

export declare function createRuntime(config?: RuntimeConfig): RomRuntime;
export declare function hasNativeBinding(): boolean;
