import { Routes } from '@angular/router';

export const routes: Routes = [
  { path: '', loadComponent: () => import('./pages/home/home').then(m => m.Home) },
  { path: 'getting-started', loadComponent: () => import('./pages/getting-started/getting-started').then(m => m.GettingStarted) },
  { path: 'api/string', loadComponent: () => import('./pages/api/string/string').then(m => m.StringApi) },
  { path: 'api/handle', loadComponent: () => import('./pages/api/handle/handle').then(m => m.HandleApi) },
  { path: 'api/process', loadComponent: () => import('./pages/api/process/process').then(m => m.ProcessApi) },
  { path: 'api/registry', loadComponent: () => import('./pages/api/registry/registry').then(m => m.RegistryApi) },
  { path: 'api/window', loadComponent: () => import('./pages/api/window/window').then(m => m.WindowApi) },
  { path: 'api/fs', loadComponent: () => import('./pages/api/fs/fs').then(m => m.FsApi) },
  { path: 'guides/safety', loadComponent: () => import('./pages/guides/safety/safety').then(m => m.SafetyGuide) },
  { path: 'guides/performance', loadComponent: () => import('./pages/guides/performance/performance').then(m => m.PerformanceGuide) },
  { path: '**', redirectTo: '' }
];
