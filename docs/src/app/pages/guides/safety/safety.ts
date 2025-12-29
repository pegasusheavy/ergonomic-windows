import { Component, OnInit } from '@angular/core';
import { Meta, Title } from '@angular/platform-browser';

@Component({
  selector: 'app-safety-guide',
  imports: [],
  templateUrl: './safety.html'
})
export class SafetyGuide implements OnInit {
  constructor(private meta: Meta, private title: Title) {}

  ngOnInit() {
    this.title.setTitle('Safety Guide - ergonomic-windows');
    this.meta.updateTag({ name: 'description', content: 'Learn about unsafe code practices in ergonomic-windows. RAII patterns, safety invariants, and how we keep Windows API calls safe.' });
    this.meta.updateTag({ name: 'keywords', content: 'rust, unsafe, safety, ffi, windows api, raii, memory safety' });
    this.meta.updateTag({ property: 'og:title', content: 'Safety Guide - ergonomic-windows' });
    this.meta.updateTag({ property: 'og:url', content: 'https://pegasusheavy.github.io/ergonomic-windows/guides/safety' });
  }
}

