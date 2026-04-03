# Tana Landing Page

This directory contains the GitHub Pages landing page for the Tana project.

## Overview

A modern, responsive single-page landing site showcasing the Tana media tracking tool. Built with vanilla HTML, CSS, and JavaScript - no build process required.

## Local Development

Open `index.html` directly in your browser, or serve with a local web server:

```bash
# Using Python 3
python -m http.server 8000

# Using Node.js http-server
npx http-server

# Using Ruby
ruby -run -ehttpd . -p8000
```

Then visit `http://localhost:8000` in your browser.

## Project Structure

```
web/
├── index.html      # Main landing page
├── README.md       # This file
└── .nojekyll       # Tells GitHub Pages not to use Jekyll
```

## Customization

Before deploying, update the GitHub links in `index.html`:

1. Replace `yourusername` with your actual GitHub username
2. Update any hardcoded links and references
3. Customize the content sections as needed:
   - Hero section headline and description
   - Feature cards
   - Tech stack items
   - Quick start instructions
   - Footer links

## Features

- ✅ Fully responsive design (mobile-first)
- ✅ Modern CSS with smooth animations
- ✅ No external dependencies
- ✅ Fast load times
- ✅ SEO optimized
- ✅ Accessibility friendly
- ✅ Smooth scroll navigation

## Deployment

The site is automatically deployed to GitHub Pages when pushed to your repository.

### GitHub Pages Setup

1. Go to your repository settings
2. Navigate to "Pages" section
3. Set "Source" to "Deploy from a branch"
4. Select the branch (main) and folder (root or /docs)
5. The site will be published at `https://yourusername.github.io/tana/`

### Custom Domain (Optional)

To use a custom domain:

1. Create a `CNAME` file in this directory with your domain
2. Update DNS records to point to GitHub Pages
3. Enable "Enforce HTTPS" in repository settings

## Color Scheme

The landing page uses a modern color palette:

- Primary: `#6366f1` (Indigo)
- Secondary: `#ec4899` (Pink)
- Accent: `#f59e0b` (Amber)
- Text: `#1f2937` (Dark Gray)
- Background: `#ffffff` (White)

You can customize these colors by editing the CSS variables in the `<style>` section of `index.html`.

## Browser Support

- Chrome/Edge (latest)
- Firefox (latest)
- Safari (latest)
- Mobile browsers

## Performance

- Single HTML file for instant loading
- Inline CSS (no external stylesheets)
- Minimal JavaScript
- Optimized for Core Web Vitals

## License

Same as Tana project (MIT / Apache 2.0)