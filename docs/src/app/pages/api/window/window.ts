import { Component, OnInit } from '@angular/core';
import { Meta, Title } from '@angular/platform-browser';

@Component({
  selector: 'app-window-api',
  imports: [],
  templateUrl: './window.html'
})
export class WindowApi implements OnInit {
  constructor(private meta: Meta, private title: Title) {}

  ngOnInit() {
    this.title.setTitle('Window API - ergonomic-windows');
    this.meta.updateTag({ name: 'description', content: 'Create native Windows GUI windows in Rust. WindowBuilder pattern with message handling via traits.' });
    this.meta.updateTag({ name: 'keywords', content: 'rust, windows, gui, window, createwindow, message loop, hwnd' });
    this.meta.updateTag({ property: 'og:title', content: 'Window API - ergonomic-windows' });
    this.meta.updateTag({ property: 'og:url', content: 'https://pegasusheavy.github.io/ergonomic-windows/api/window' });
  }
}

