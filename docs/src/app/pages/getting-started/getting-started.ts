import { Component, OnInit } from '@angular/core';
import { RouterLink } from '@angular/router';
import { Meta, Title } from '@angular/platform-browser';

@Component({
  selector: 'app-getting-started',
  imports: [RouterLink],
  templateUrl: './getting-started.html'
})
export class GettingStarted implements OnInit {
  constructor(private meta: Meta, private title: Title) {}

  ngOnInit() {
    this.title.setTitle('Getting Started - ergonomic-windows');
    this.meta.updateTag({ name: 'description', content: 'Learn how to install and use ergonomic-windows in your Rust project. Quick start guide with examples for Windows API wrappers.' });
    this.meta.updateTag({ property: 'og:title', content: 'Getting Started with ergonomic-windows' });
    this.meta.updateTag({ property: 'og:url', content: 'https://pegasusheavy.github.io/ergonomic-windows/getting-started' });
  }
}

