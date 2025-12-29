import { Component, OnInit } from '@angular/core';
import { Meta, Title } from '@angular/platform-browser';

@Component({
  selector: 'app-registry-api',
  imports: [],
  templateUrl: './registry.html'
})
export class RegistryApi implements OnInit {
  constructor(private meta: Meta, private title: Title) {}

  ngOnInit() {
    this.title.setTitle('Registry API - ergonomic-windows');
    this.meta.updateTag({ name: 'description', content: 'Read and write Windows Registry keys and values in Rust. Type-safe access to DWORD, QWORD, String, Binary, and MultiString values.' });
    this.meta.updateTag({ name: 'keywords', content: 'rust, windows, registry, hkey, regedit, registry key, registry value, dword, qword' });
    this.meta.updateTag({ property: 'og:title', content: 'Registry API - ergonomic-windows' });
    this.meta.updateTag({ property: 'og:url', content: 'https://pegasusheavy.github.io/ergonomic-windows/api/registry' });
  }
}

