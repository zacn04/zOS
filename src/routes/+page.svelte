<script lang="ts">
  import { goto } from "$app/navigation";
</script>

<div class="home-container">
  <div class="header">
    <h1 class="app-title">zOS</h1>
    <p class="app-subtitle">zac's operating space</p>
  </div>

  <div class="portals-grid">
    <button class="portal-card portal-solve" on:click={() => goto("/solve")}>
      <div class="portal-icon">🧩</div>
      <h2 class="portal-title">Solve</h2>
      <p class="portal-description">Work on adaptive problems across RL, ML, coding, and math</p>
    </button>

    <button class="portal-card portal-learn" on:click={() => goto("/learn")}>
      <div class="portal-icon">📚</div>
      <h2 class="portal-title">Learn</h2>
      <p class="portal-description">Explore problems by topic and practice specific skills</p>
    </button>

    <button class="portal-card portal-improve" on:click={() => goto("/improve")}>
      <div class="portal-icon">📈</div>
      <h2 class="portal-title">Improve</h2>
      <p class="portal-description">View your skills, track progress, and get personalized coaching</p>
    </button>
  </div>
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    min-height: 100vh;
  }

  .home-container {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 40px 20px;
    width: 100%;
    margin: 0 auto;
  }

  .header {
    text-align: center;
    margin-bottom: 60px;
    color: white;
  }

  .app-title {
    font-size: 64px;
    font-weight: 700;
    margin: 0 0 12px 0;
    text-shadow: 0 2px 10px rgba(0, 0, 0, 0.2);
    letter-spacing: -1px;
  }

  .app-subtitle {
    font-size: 20px;
    margin: 0;
    opacity: 0.9;
    font-weight: 300;
  }

  .portals-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 24px;
    width: 100%;
    padding: 0 20px;
  }

  .portal-card {
    background: white;
    border: none;
    border-radius: 16px;
    padding: 2vw 1.5vw;
    cursor: pointer;
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
    text-align: center;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1vw;
    min-height: 120px;
    width: 100%;
  }

  .portal-card:hover {
    transform: translateY(-8px);
    box-shadow: 0 12px 24px rgba(0, 0, 0, 0.15);
  }

  .portal-card:active {
    transform: translateY(-4px);
  }

  .portal-icon {
    font-size: clamp(28px, 3vw, 48px);
    margin-bottom: 4px;
  }

  .portal-title {
    font-size: clamp(18px, 2vw, 28px);
    font-weight: 600;
    margin: 0;
    color: #1a1a1a;
  }

  .portal-description {
    font-size: clamp(11px, 1.2vw, 14px);
    color: #666;
    margin: 0;
    line-height: 1.4;
  }

  .portal-solve {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: white;
  }

  .portal-solve .portal-title,
  .portal-solve .portal-description {
    color: white;
  }

  .portal-solve .portal-description {
    opacity: 0.9;
  }

  .portal-learn {
    background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
    color: white;
  }

  .portal-learn .portal-title,
  .portal-learn .portal-description {
    color: white;
  }

  .portal-learn .portal-description {
    opacity: 0.9;
  }

  .portal-improve {
    background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
    color: white;
  }

  .portal-improve .portal-title,
  .portal-improve .portal-description {
    color: white;
  }

  .portal-improve .portal-description {
    opacity: 0.9;
  }

  /* Dark mode */
  @media (prefers-color-scheme: dark) {
    :global(body) {
      background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
    }

    .portal-card {
      background: #2d2d2d;
    }

    .portal-card:not(.portal-solve):not(.portal-learn):not(.portal-improve) {
      background: #2d2d2d;
    }

    .portal-card:not(.portal-solve):not(.portal-learn):not(.portal-improve) .portal-title {
      color: #e0e0e0;
    }

    .portal-card:not(.portal-solve):not(.portal-learn):not(.portal-improve) .portal-description {
      color: #b0b0b0;
    }
  }

  /* Responsive */
  @media (max-width: 768px) {
    .app-title {
      font-size: 48px;
    }

    .app-subtitle {
      font-size: 18px;
    }

    .portals-grid {
      grid-template-columns: 1fr;
      gap: 20px;
    }

    .portal-card {
      padding: 24px 20px;
      min-height: 180px;
    }

    .portal-icon {
      font-size: 40px;
    }

    .portal-title {
      font-size: 24px;
    }

    .portal-description {
      font-size: 14px;
    }
  }
</style>
