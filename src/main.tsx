import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { StrictMode, type ReactElement } from 'react';
import { createRoot } from 'react-dom/client';
import type { ReadAloudGateway } from './read-aloud/application/ports.ts';
import { ReadAloudStore } from './read-aloud/application/read-aloud-store.ts';
import { VoiceStore } from './read-aloud/application/voice-store.ts';
import { FakeReadAloudGateway } from './read-aloud/infrastructure/fake-read-aloud-gateway.ts';
import { LocalStorageVoicePreferences } from './read-aloud/infrastructure/local-storage-voice-preferences.ts';
import { TauriReadAloudGateway } from './read-aloud/infrastructure/tauri-read-aloud-gateway.ts';
import { ReadAloudPanel } from './read-aloud/ui/read-aloud-panel.tsx';
import { TaskStore } from './tasks/application/task-store.ts';
import { CryptoIdGenerator } from './tasks/infrastructure/crypto-id-generator.ts';
import { LocalStorageTaskRepository } from './tasks/infrastructure/local-storage-task-repository.ts';
import { TaskApp } from './tasks/ui/task-app.tsx';
import './styles.css';

const container = document.getElementById('root');
if (container === null) {
  throw new Error('index.html has no #root element.');
}

const params = new URLSearchParams(window.location.search);

const inBrowserDev = import.meta.env.DEV && !isTauri();

function readAloudGateway(): ReadAloudGateway {
  return inBrowserDev
    ? new FakeReadAloudGateway(params.get('selection'))
    : new TauriReadAloudGateway(invoke, listen);
}

function app(): ReactElement {
  if (params.get('view') === 'read-aloud') {
    const gateway = readAloudGateway();
    const voices = new VoiceStore({
      catalog: gateway,
      preferences: new LocalStorageVoicePreferences(window.localStorage),
      preferredLanguage: navigator.language,
    });
    voices.load();
    return <ReadAloudPanel store={new ReadAloudStore(gateway, voices.current)} voices={voices} />;
  }
  const store = new TaskStore({
    repository: new LocalStorageTaskRepository(window.localStorage),
    ids: new CryptoIdGenerator(),
  });
  return <TaskApp store={store} />;
}

createRoot(container).render(<StrictMode>{app()}</StrictMode>);
