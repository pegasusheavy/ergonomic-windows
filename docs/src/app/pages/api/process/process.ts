import { Component, OnInit } from '@angular/core';
import { Meta, Title } from '@angular/platform-browser';

@Component({
  selector: 'app-process-api',
  imports: [],
  templateUrl: './process.html'
})
export class ProcessApi implements OnInit {
  constructor(private meta: Meta, private title: Title) {}

  ngOnInit() {
    this.title.setTitle('Process API - ergonomic-windows');
    this.meta.updateTag({ name: 'description', content: 'Create, manage, and query Windows processes in Rust. Fluent builder API for process spawning, waiting, and termination.' });
    this.meta.updateTag({ name: 'keywords', content: 'rust, windows, process, spawn, createprocess, process management, command' });
    this.meta.updateTag({ property: 'og:title', content: 'Process API - ergonomic-windows' });
    this.meta.updateTag({ property: 'og:url', content: 'https://pegasusheavy.github.io/ergonomic-windows/api/process' });
  }
}

