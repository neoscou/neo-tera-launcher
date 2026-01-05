/**
 * Resize Handler for Neolithic TERA Launcher
 * Handles window resizing and scaling of UI elements
 */

const ResizeHandler = {
  // Base dimensions (original design dimensions)
  BASE_WIDTH: 1282,
  BASE_HEIGHT: 759,
  MIN_WIDTH: 1280,
  MIN_HEIGHT: 1024,
  
  // Current scale factors
  scaleX: 1,
  scaleY: 1,
  
  /**
   * Initialize the resize handler
   */
  init() {
    this.updateScale();
    this.setupResizeListener();
    this.applyResponsiveStyles();
  },
  
  /**
   * Calculate and update scale factors based on current window size
   */
  updateScale() {
    const width = window.innerWidth;
    const height = window.innerHeight;
    
    // Calculate scale factors
    this.scaleX = width / this.BASE_WIDTH;
    this.scaleY = height / this.BASE_HEIGHT;
    
    // Use the minimum scale to maintain aspect ratio awareness
    const uniformScale = Math.min(this.scaleX, this.scaleY);
    
    // Apply CSS custom properties for scaling
    document.documentElement.style.setProperty('--scale-x', this.scaleX);
    document.documentElement.style.setProperty('--scale-y', this.scaleY);
    document.documentElement.style.setProperty('--uniform-scale', uniformScale);
    document.documentElement.style.setProperty('--window-width', width + 'px');
    document.documentElement.style.setProperty('--window-height', height + 'px');
    
    // Calculate font scale (don't scale fonts as aggressively)
    const fontScale = Math.max(0.7, Math.min(1.3, uniformScale));
    document.documentElement.style.setProperty('--font-scale', fontScale);
  },
  
  /**
   * Setup window resize listener
   */
  setupResizeListener() {
    let resizeTimer;
    
    window.addEventListener('resize', () => {
      // Debounce resize events
      clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => {
        this.updateScale();
        this.onResize();
      }, 16); // ~60fps
    });
  },
  
  /**
   * Apply responsive CSS styles dynamically
   */
  applyResponsiveStyles() {
    const style = document.createElement('style');
    style.id = 'resize-handler-styles';
    style.textContent = `
      /* Responsive font sizing */
      body {
        font-size: calc(14px * var(--font-scale, 1));
      }
      
      /* Scale modal elements */
      .modal-content {
        max-width: min(90vw, calc(600px * var(--uniform-scale, 1)));
        max-height: 90vh;
      }
      
      /* Scale buttons proportionally */
      button, .btn {
        font-size: calc(14px * var(--font-scale, 1));
        padding: calc(8px * var(--uniform-scale, 1)) calc(16px * var(--uniform-scale, 1));
      }
      
      /* Responsive header */
      .header1 {
        font-size: calc(18px * var(--font-scale, 1));
        padding: calc(5px * var(--uniform-scale, 1)) calc(15px * var(--uniform-scale, 1)) calc(8px * var(--uniform-scale, 1)) calc(250px * var(--scale-x, 1));
      }
      
      /* Scale icon buttons */
      .titlebar-button, .btn-minimize1, .btn-close1 {
        width: calc(30px * var(--uniform-scale, 1));
        height: calc(30px * var(--uniform-scale, 1));
      }
      
      .titlebar-button svg {
        width: calc(16px * var(--uniform-scale, 1));
        height: calc(16px * var(--uniform-scale, 1));
      }
      
      /* Responsive spacing */
      .app-btn-content {
        gap: calc(18px * var(--uniform-scale, 1));
      }
      
      /* Input fields */
      input, select, textarea {
        font-size: calc(14px * var(--font-scale, 1));
        padding: calc(8px * var(--uniform-scale, 1));
      }
      
      /* Progress bars */
      .progress-bar, .loading-bar {
        height: calc(4px * var(--uniform-scale, 1));
      }
      
      /* User panel icons scale */
      .user-icon-one, .user-icon-two {
        transform: scale(var(--uniform-scale, 1));
      }
      
      /* Dropdown panel responsive sizing */
      .dropdown-panel {
        font-size: calc(14px * var(--font-scale, 1));
      }
      
      .dropdown-panel .menu-item {
        font-size: calc(13px * var(--font-scale, 1));
        padding: calc(10px * var(--uniform-scale, 1)) calc(15px * var(--uniform-scale, 1));
      }
      
      /* News slider responsive adjustments */
      .swiper-button-prev, .swiper-button-next {
        transform: scale(var(--uniform-scale, 1));
      }
      
      .swiper-pagination {
        bottom: calc(10px * var(--uniform-scale, 1));
      }
      
      /* Adjust minimum sizes at smaller scales */
      @media (max-width: 1000px) {
        .header1 {
          padding-left: calc(150px * var(--scale-x, 1));
        }
      }
      
      @media (max-width: 800px) {
        .header1 {
          padding-left: calc(20px * var(--scale-x, 1));
          gap: calc(50px * var(--uniform-scale, 1));
        }
      }
    `;
    
    // Remove existing style if present
    const existingStyle = document.getElementById('resize-handler-styles');
    if (existingStyle) {
      existingStyle.remove();
    }
    
    document.head.appendChild(style);
  },
  
  /**
   * Callback for resize events (can be extended by app)
   */
  onResize() {
    // Dispatch custom event for other parts of the app to listen to
    window.dispatchEvent(new CustomEvent('launcher-resize', {
      detail: {
        scaleX: this.scaleX,
        scaleY: this.scaleY,
        width: window.innerWidth,
        height: window.innerHeight
      }
    }));
  },
  
  /**
   * Get current scale information
   */
  getScaleInfo() {
    return {
      scaleX: this.scaleX,
      scaleY: this.scaleY,
      width: window.innerWidth,
      height: window.innerHeight,
      baseWidth: this.BASE_WIDTH,
      baseHeight: this.BASE_HEIGHT
    };
  }
};

// Auto-initialize when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', () => ResizeHandler.init());
} else {
  ResizeHandler.init();
}

// Export for use in other modules
window.ResizeHandler = ResizeHandler;
