import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { StrictMode, type ReactElement } from 'react';
import { createRoot } from 'react-dom/client';
import './styles.css';

const container = document.getElementById('root');
if (container === null) {
  throw new Error('index.html has no #root element.');
}

const params = new URLSearchParams(window.location.search);

const inBrowserDev = import.meta.env.DEV && !isTauri();

async function overlayView(): Promise<ReactElement> {
  const [store, motion, gateway, ui] = await Promise.all([
    import('./overlay/application/overlay-store.ts'),
    import('./overlay/infrastructure/media-query-motion.ts'),
    import('./overlay/infrastructure/tauri-overlay-gateway.ts'),
    import('./overlay/ui/overlay-view.tsx'),
  ]);
  const ipc = inBrowserDev
    ? new (await import('./overlay/infrastructure/window-event-ipc.ts')).WindowEventIpc(window)
    : { invoke, listen };
  const overlay = new store.OverlayStore(
    new gateway.TauriOverlayGateway(ipc.invoke, ipc.listen),
    new motion.MediaQueryMotion(window.matchMedia(motion.REDUCED_MOTION_QUERY)),
  );
  return <ui.OverlayView store={overlay} />;
}

async function settingsView(): Promise<ReactElement> {
  const [store, gateway, ui] = await Promise.all([
    import('./settings/application/settings-store.ts'),
    import('./settings/infrastructure/tauri-settings-gateway.ts'),
    import('./settings/ui/settings-window.tsx'),
  ]);
  const settings = new store.SettingsStore(
    inBrowserDev
      ? new (
          await import('./settings/infrastructure/fake-settings-gateway.ts')
        ).FakeSettingsGateway()
      : new gateway.TauriSettingsGateway(invoke, listen),
  );
  settings.load();
  return <ui.SettingsWindow store={settings} />;
}

async function readAloudView(): Promise<ReactElement> {
  const [readAloud, adapters] = await Promise.all([
    import('./read-aloud/index.ts'),
    import('./read-aloud/infrastructure/adapters.ts'),
  ]);
  const { ui } = readAloud;
  const ipc = inBrowserDev
    ? new (await import('./read-aloud/infrastructure/fake-read-aloud-ipc.ts')).FakeReadAloudIpc(
        window,
        params.get('selection'),
      )
    : { invoke, listen };
  const readAloudGateway = new adapters.TauriReadAloudGateway(ipc.invoke, ipc.listen);
  const voices = new readAloud.VoiceStore({
    catalog: readAloudGateway,
    preferences: new adapters.LocalStorageVoicePreferences(window.localStorage),
    preferredLanguage: navigator.language,
  });
  voices.load();
  const store = new readAloud.ReaderStore({
    gateway: readAloudGateway,
    clock: new adapters.AnimationFrameClock(window),
    preferences: new adapters.LocalStorageReaderPreferences(window.localStorage),
    currentVoice: voices.current,
  });
  return <ui.ReaderWindow store={store} voices={voices} />;
}

async function tasksView(): Promise<ReactElement> {
  const [store, ids, repository, ui] = await Promise.all([
    import('./tasks/application/task-store.ts'),
    import('./tasks/infrastructure/crypto-id-generator.ts'),
    import('./tasks/infrastructure/local-storage-task-repository.ts'),
    import('./tasks/ui/task-app.tsx'),
  ]);
  const tasks = new store.TaskStore({
    repository: new repository.LocalStorageTaskRepository(window.localStorage),
    ids: new ids.CryptoIdGenerator(),
  });
  return <ui.TaskApp store={tasks} />;
}

const VIEWS: Readonly<Partial<Record<string, () => Promise<ReactElement>>>> = {
  overlay: overlayView,
  settings: settingsView,
  'read-aloud': readAloudView,
};

const view = VIEWS[params.get('view') ?? ''] ?? tasksView;

createRoot(container).render(<StrictMode>{await view()}</StrictMode>);
