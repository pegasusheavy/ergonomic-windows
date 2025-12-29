import { Component } from '@angular/core';
import { RouterOutlet, RouterLink, RouterLinkActive } from '@angular/router';
import { FontAwesomeModule } from '@fortawesome/angular-fontawesome';
import { 
  faGithub, 
  faRust 
} from '@fortawesome/free-brands-svg-icons';
import { 
  faBars, 
  faBook, 
  faCode, 
  faCog, 
  faFileAlt, 
  faHome, 
  faRocket, 
  faShieldAlt, 
  faTachometerAlt,
  faTerminal,
  faDatabase,
  faFolderOpen,
  faWindowMaximize,
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
  faBook = faBook;
  faCode = faCode;
  faCog = faCog;
  faFileAlt = faFileAlt;
  faHome = faHome;
  faRocket = faRocket;
  faShieldAlt = faShieldAlt;
  faTachometerAlt = faTachometerAlt;
  faTerminal = faTerminal;
  faDatabase = faDatabase;
  faFolderOpen = faFolderOpen;
  faWindowMaximize = faWindowMaximize;
  faExternalLinkAlt = faExternalLinkAlt;
  faHeart = faHeart;

  toggleSidebar() {
    this.sidebarOpen = !this.sidebarOpen;
  }
}
