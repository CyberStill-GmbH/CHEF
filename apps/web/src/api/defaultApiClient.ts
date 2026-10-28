import { isTauri } from '@tauri-apps/api/core';

import { FixtureApiClient } from './FixtureApiClient';
import { TauriApiClient } from './TauriApiClient';

export function createDefaultApiClient() {
  return isTauri() ? new TauriApiClient() : new FixtureApiClient();
}
