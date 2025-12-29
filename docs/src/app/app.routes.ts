import { Routes } from '@angular/router';

export const routes: Routes = [
  { path: '', loadComponent: () => import('./pages/home/home').then(m => m.Home) },
  { path: 'getting-started', loadComponent: () => import('./pages/getting-started/getting-started').then(m => m.GettingStarted) },
  // Core modules
  { path: 'api/string', loadComponent: () => import('./pages/api/string/string').then(m => m.StringApi) },
  { path: 'api/handle', loadComponent: () => import('./pages/api/handle/handle').then(m => m.HandleApi) },
  { path: 'api/process', loadComponent: () => import('./pages/api/process/process').then(m => m.ProcessApi) },
  { path: 'api/registry', loadComponent: () => import('./pages/api/registry/registry').then(m => m.RegistryApi) },
  { path: 'api/window', loadComponent: () => import('./pages/api/window/window').then(m => m.WindowApi) },
  { path: 'api/fs', loadComponent: () => import('./pages/api/fs/fs').then(m => m.FsApi) },
  // Extended modules
  { path: 'api/thread', loadComponent: () => import('./pages/api/thread/thread').then(m => m.ThreadApi) },
  { path: 'api/mem', loadComponent: () => import('./pages/api/mem/mem').then(m => m.MemApi) },
  { path: 'api/console', loadComponent: () => import('./pages/api/console/console').then(m => m.ConsoleApi) },
  { path: 'api/env', loadComponent: () => import('./pages/api/env/env').then(m => m.EnvApi) },
  { path: 'api/pipe', loadComponent: () => import('./pages/api/pipe/pipe').then(m => m.PipeApi) },
  { path: 'api/time', loadComponent: () => import('./pages/api/time/time').then(m => m.TimeApi) },
  { path: 'api/module', loadComponent: () => import('./pages/api/module/module').then(m => m.ModuleApi) },
  { path: 'api/sysinfo', loadComponent: () => import('./pages/api/sysinfo/sysinfo').then(m => m.SysinfoApi) },
  { path: 'api/security', loadComponent: () => import('./pages/api/security/security').then(m => m.SecurityApi) },
  // Guides
  { path: 'guides/safety', loadComponent: () => import('./pages/guides/safety/safety').then(m => m.SafetyGuide) },
  { path: 'guides/performance', loadComponent: () => import('./pages/guides/performance/performance').then(m => m.PerformanceGuide) },
  { path: '**', redirectTo: '' }
];
