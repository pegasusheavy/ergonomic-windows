import { Component, OnInit } from '@angular/core';
import { Meta, Title } from '@angular/platform-browser';

@Component({
  selector: 'app-fs-api',
  imports: [],
  templateUrl: './fs.html'
})
export class FsApi implements OnInit {
  constructor(private meta: Meta, private title: Title) {}

  ngOnInit() {
    this.title.setTitle('File System API - ergonomic-windows');
    this.meta.updateTag({ name: 'description', content: 'Windows file system operations in Rust. File attributes, move/delete operations, system directories, and more.' });
    this.meta.updateTag({ name: 'keywords', content: 'rust, windows, filesystem, file attributes, movefile, deletefile, system directory' });
    this.meta.updateTag({ property: 'og:title', content: 'File System API - ergonomic-windows' });
    this.meta.updateTag({ property: 'og:url', content: 'https://pegasusheavy.github.io/ergonomic-windows/api/fs' });
  }
}

