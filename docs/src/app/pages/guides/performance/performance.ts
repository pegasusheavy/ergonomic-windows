import { Component, OnInit } from '@angular/core';
import { Meta, Title } from '@angular/platform-browser';

@Component({
  selector: 'app-performance-guide',
  imports: [],
  templateUrl: './performance.html'
})
export class PerformanceGuide implements OnInit {
  constructor(private meta: Meta, private title: Title) {}

  ngOnInit() {
    this.title.setTitle('Performance Guide - ergonomic-windows');
    this.meta.updateTag({ name: 'description', content: 'Performance optimizations in ergonomic-windows. Small string optimization, object pooling, benchmarks, and allocation analysis.' });
    this.meta.updateTag({ name: 'keywords', content: 'rust, performance, benchmark, allocation, small string optimization, object pool' });
    this.meta.updateTag({ property: 'og:title', content: 'Performance Guide - ergonomic-windows' });
    this.meta.updateTag({ property: 'og:url', content: 'https://pegasusheavy.github.io/ergonomic-windows/guides/performance' });
  }
}

