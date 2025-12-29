import { Component } from '@angular/core';
import { RouterOutlet, RouterLink, RouterLinkActive } from '@angular/router';
import { FontAwesomeModule } from '@fortawesome/angular-fontawesome';
import { faGithub, faRust } from '@fortawesome/free-brands-svg-icons';
import {
  faBars,
  faHome,
  faRocket,
  faShieldAlt,
  faTachometerAlt,
  faExternalLinkAlt,
  faHeart
} from '@fortawesome/free-solid-svg-icons';

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, RouterLink, RouterLinkActive, FontAwesomeModule],
  templateUrl: './app.html',
  styleUrl: './app.css'
})
export class App {
  sidebarOpen = false;

  // Brand icons
  faGithub = faGithub;
  faRust = faRust;

  // Solid icons
  faBars = faBars;
  faHome = faHome;
  faRocket = faRocket;
  faShieldAlt = faShieldAlt;
  faTachometerAlt = faTachometerAlt;
  faExternalLinkAlt = faExternalLinkAlt;
  faHeart = faHeart;

  toggleSidebar() {
    this.sidebarOpen = !this.sidebarOpen;
  }
}
