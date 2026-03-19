export interface RuntimeConfig {
  href?: string;
  user_agent?: string;
  app_name?: string;
  platform?: string;
  language?: string;
  languages?: string[];
  hardware_concurrency?: number;
  device_memory?: number;
  webdriver?: boolean;
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
