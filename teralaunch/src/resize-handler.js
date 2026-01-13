/**
 * Resize Handler for Neolithic TERA Launcher
 * Handles window resizing and scaling of UI elements
 */

const ResizeHandler = {
  // Base dimensions (original design dimensions)
  BASE_WIDTH: 1282,
  BASE_HEIGHT: 759,
  MIN_WIDTH: 1366,
  MIN_HEIGHT: 960,
  
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
      
      /* News block responsive scaling */
      .news-block {
        min-height: max(150px, calc(200px * var(--uniform-scale, 1)));
      }
      
      .news-block-parent {
        padding-top: max(40px, calc(130px * var(--uniform-scale, 1)));
      }
      
      /* Swiper slide content scaling */
      .swiper-slide img {
        height: max(120px, calc(180px * var(--uniform-scale, 1)));
      }
      
      .slide-content h3 {
        font-size: max(14px, calc(18px * var(--font-scale, 1)));
      }
      
      .slide-content p {
        font-size: max(11px, calc(13px * var(--font-scale, 1)));
      }
      
      /* Adjust minimum sizes at smaller scales */
      @media (max-width: 1000px) {
        .header1 {
          padding-left: calc(150px * var(--scale-x, 1));
        }
        
        .news-block {
          min-height: 140px;
        }
      }
      
      @media (max-width: 800px) {
        /* Prevent overflow on main containers */
        .content {
          padding: 0 1% !important;
          overflow: visible !important;
        }
        
        .home-container {
          overflow: visible !important;
          width: 100% !important;
        }
        
        .header1 {
          padding-left: 10px !important;
          padding-right: 10px !important;
          gap: 20px !important;
        }
        
        /* Ensure home-wrapper doesn't hide content */
        .home-wrapper {
          padding: 0 !important;
          gap: 6px !important;
          flex-wrap: nowrap !important;
          width: 100% !important;
          overflow: visible !important;
        }
        
        .home-content {
          flex: 0 1 auto !important;
          max-width: calc(100% - 120px) !important;
          min-width: 0 !important;
          gap: 6px !important;
        }
        
        /* News carousel - compact container */
        .news-block {
          min-height: 95px !important;
          max-height: 95px !important;
          height: 95px !important;
          padding: 6px 8px !important;
          overflow: hidden !important;
          display: flex !important;
          flex-direction: column !important;
        }
        
        .news-block-parent {
          padding-top: 10px !important;
          min-height: auto !important;
          flex: 1 !important;
        }
        
        .logo-wrapper {
          gap: 2px !important;
          margin-bottom: 4px !important;
        }
        
        .swiper-container {
          height: 80px !important;
          flex: 1 !important;
        }
        
        .swiper-slide {
          display: flex !important;
          align-items: center !important;
          height: 80px !important;
          padding: 2px !important;
        }
        
        .swiper-slide img {
          height: 65px !important;
          max-height: 65px !important;
          width: auto !important;
          max-width: 100px !important;
          object-fit: contain !important;
          margin-right: 6px !important;
          flex-shrink: 0 !important;
        }
        
        .slide-content {
          padding: 2px 4px !important;
          flex: 1 !important;
          min-width: 0 !important;
        }
        
        .slide-content h3 {
          font-size: 10px !important;
          margin: 0 0 2px 0 !important;
          line-height: 1.2 !important;
          overflow: hidden !important;
          text-overflow: ellipsis !important;
        }
        
        .slide-content p {
          font-size: 8.5px !important;
          line-height: 1.2 !important;
          margin: 0 !important;
          overflow: hidden !important;
          display: -webkit-box !important;
          -webkit-line-clamp: 2 !important;
          -webkit-box-orient: vertical !important;
        }
        
        .swiper-button-prev, .swiper-button-next {
          transform: scale(0.5) !important;
        }
        
        .swiper-pagination {
          bottom: 1px !important;
          transform: scale(0.7) !important;
        }
        
        /* Account Info - ensure visibility with fixed positioning */
        .user-panel {
          position: fixed !important;
          top: 10px !important;
          right: 10px !important;
          z-index: 1000 !important;
          flex: none !important;
          width: auto !important;
          margin: 0 !important;
        }
        
        .user-panel-header {
          gap: 5px !important;
          flex-shrink: 0 !important;
          justify-content: flex-end !important;
        }
        
        .account-info-label {
          font-size: 9px !important;
          letter-spacing: 0.3px !important;
          white-space: nowrap !important;
          display: inline-block !important;
          opacity: 1 !important;
        }
        
        .btn-user-avatar {
          width: 28px !important;
          height: 28px !important;
          flex-shrink: 0 !important;
        }
        
        .user-icon-one {
          width: 7px !important;
          height: 7px !important;
        }
        
        .user-icon-two {
          width: 14px !important;
          height: 5px !important;
        }
        
        .dropdown-panel-wrapper {
          right: 0 !important;
          top: 100% !important;
          margin-top: 5px !important;
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
