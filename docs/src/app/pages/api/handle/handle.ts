import { Component, OnInit } from '@angular/core';
import { Meta, Title } from '@angular/platform-browser';

@Component({
  selector: 'app-handle-api',
  imports: [],
  templateUrl: './handle.html'
})
export class HandleApi implements OnInit {
  constructor(private meta: Meta, private title: Title) {}

  ngOnInit() {
    this.title.setTitle('Handle API - ergonomic-windows');
    this.meta.updateTag({ name: 'description', content: 'RAII wrappers for Windows handles in Rust. OwnedHandle and BorrowedHandle for automatic resource management and leak prevention.' });
    this.meta.updateTag({ name: 'keywords', content: 'rust, windows, handle, raii, ownedhandle, closehandle, resource management' });
    this.meta.updateTag({ property: 'og:title', content: 'Handle API - ergonomic-windows' });
    this.meta.updateTag({ property: 'og:url', content: 'https://pegasusheavy.github.io/ergonomic-windows/api/handle' });
  }
}

