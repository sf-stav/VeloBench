import { Routes } from '@angular/router';

export const routes: Routes = [
  { path: '', redirectTo: 'chat', pathMatch: 'full' },
  {
    path: 'chat',
    loadComponent: () => import('./components/chat/chat.component').then((m) => m.ChatComponent),
  },
  {
    // Report-only page: opened via "View" on the Sessions page.
    path: 'analytics/:session',
    loadComponent: () =>
      import('./components/analytics/analytics.component').then((m) => m.AnalyticsComponent),
  },
  {
    // The analytics list was consolidated into Sessions.
    path: 'analytics',
    redirectTo: 'sessions',
    pathMatch: 'full',
  },
  {
    path: 'sessions',
    loadComponent: () =>
      import('./components/benchmarks/benchmarks.component').then((m) => m.BenchmarksComponent),
  },
  {
    // Saved comparisons list + creation.
    path: 'comparisons',
    loadComponent: () =>
      import('./components/comparisons/comparisons.component').then((m) => m.ComparisonsComponent),
  },
  {
    // One comparison report (A vs B deltas).
    path: 'compare/:id',
    loadComponent: () =>
      import('./components/compare/compare.component').then((m) => m.CompareComponent),
  },
  {
    // Telemetry: live OTLP receiver view (mini chat + Live Stats per stream).
    path: 'telemetry',
    loadComponent: () => import('./components/telemetry/telemetry.component').then((m) => m.TelemetryComponent),
  },
  {
    // Concurrent runner (parallel load).
    path: 'runner',
    loadComponent: () => import('./components/runner/runner.component').then((m) => m.RunnerComponent),
  },
  {
    path: 'settings',
    loadComponent: () =>
      import('./components/settings/settings.component').then((m) => m.SettingsComponent),
  },
  {
    path: 'tests',
    loadComponent: () => import('./components/tests/tests.component').then((m) => m.TestsComponent),
  },
  { path: '**', redirectTo: 'chat' },
];
