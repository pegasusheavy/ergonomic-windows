import { Component, OnInit } from '@angular/core';
import { Meta, Title } from '@angular/platform-browser';

@Component({
  selector: 'app-string-api',
  imports: [],
  templateUrl: './string.html'
})
export class StringApi implements OnInit {
  constructor(private meta: Meta, private title: Title) {}

  ngOnInit() {
    this.title.setTitle('String API - ergonomic-windows');
    this.meta.updateTag({ name: 'description', content: 'UTF-8 to UTF-16 string conversion for Windows APIs in Rust. WideString with small string optimization, object pooling, and zero-cost abstractions.' });
    this.meta.updateTag({ name: 'keywords', content: 'rust, windows, utf16, utf8, string conversion, widestring, pcwstr, small string optimization' });
    this.meta.updateTag({ property: 'og:title', content: 'String API - ergonomic-windows' });
    this.meta.updateTag({ property: 'og:url', content: 'https://pegasusheavy.github.io/ergonomic-windows/api/string' });
  }
}

