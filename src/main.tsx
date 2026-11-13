import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { TaskStore } from './tasks/application/task-store.ts';
import { CryptoIdGenerator } from './tasks/infrastructure/crypto-id-generator.ts';
import { LocalStorageTaskRepository } from './tasks/infrastructure/local-storage-task-repository.ts';
import { TaskApp } from './tasks/ui/task-app.tsx';
import './styles.css';

const container = document.getElementById('root');
if (container === null) {
  throw new Error('index.html has no #root element.');
}

const store = new TaskStore({
  repository: new LocalStorageTaskRepository(window.localStorage),
  ids: new CryptoIdGenerator(),
});

createRoot(container).render(
  <StrictMode>
    <TaskApp store={store} />
  </StrictMode>,
);
