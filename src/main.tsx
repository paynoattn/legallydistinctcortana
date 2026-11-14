import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { StrictMode, type ReactElement } from 'react';
import { createRoot } from 'react-dom/client';
import type { ReadAloudGateway } from './read-aloud/application/ports.ts';
import { ReadAloudStore } from './read-aloud/application/read-aloud-store.ts';
import { FakeReadAloudGateway } from './read-aloud/infrastructure/fake-read-aloud-gateway.ts';
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

function readAloudGateway(): ReadAloudGateway {
  return isTauri()
    ? new TauriReadAloudGateway(invoke, listen)
    : new FakeReadAloudGateway(params.get('selection'));
}

function app(): ReactElement {
  if (params.get('view') === 'read-aloud') {
    return <ReadAloudPanel store={new ReadAloudStore(readAloudGateway())} />;
  }
  const store = new TaskStore({
    repository: new LocalStorageTaskRepository(window.localStorage),
    ids: new CryptoIdGenerator(),
  });
  return <TaskApp store={store} />;
}

createRoot(container).render(<StrictMode>{app()}</StrictMode>);
