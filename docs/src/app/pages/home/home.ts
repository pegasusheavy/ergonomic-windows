import { Component, OnInit } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Meta, Title } from '@angular/platform-browser';

@Component({
  selector: 'app-home',
  imports: [RouterLink],
  templateUrl: './home.html'
})
export class Home implements OnInit {
  constructor(private meta: Meta, private title: Title) {}

  ngOnInit() {
    this.title.setTitle('ergonomic-windows - Ergonomic Windows API Wrappers for Rust');

    this.meta.updateTag({ name: 'description', content: 'Ergonomic, safe, and efficient wrappers around Windows APIs for Rust. String conversions with small string optimization, RAII handle management, process control, registry access, and more.' });
    this.meta.updateTag({ name: 'keywords', content: 'rust, windows, win32, api, ffi, ergonomic, handle, string, utf16, registry, process, safe' });
    this.meta.updateTag({ property: 'og:title', content: 'ergonomic-windows - Ergonomic Windows API Wrappers for Rust' });
    this.meta.updateTag({ property: 'og:description', content: 'Safe, ergonomic Rust wrappers for Windows APIs. RAII handles, UTF-16 strings with SSO, process management, registry access.' });
    this.meta.updateTag({ property: 'og:url', content: 'https://pegasusheavy.github.io/ergonomic-windows/' });
  }
}

