# 🎨 Custom Color Palette

## Color Scheme

The Web UI now uses your custom teal color palette:

### Primary Colors

| Color | Hex Code | Usage |
|-------|----------|-------|
| **Dark Teal** | `#005461` | Primary dark color - Headers, titles, borders |
| **Medium Teal** | `#018790` | Primary medium - Gradients, accents |
| **Light Teal** | `#00B7B5` | Accent color - Highlights, borders, buttons |
| **Light Gray** | `#F4F4F4` | Background - Section headers, neutral areas |

## Applied To

### 🎨 **Background Gradient**
- Body background: `#005461` → `#018790`
- Creates a smooth teal gradient across the entire page

### 📝 **Headers & Titles**
- Main title (h1): `#005461` (Dark Teal)
- Subtitle: `#018790` (Medium Teal)
- Card titles: `#005461` (Dark Teal)
- Card title borders: `#00B7B5` (Light Teal)

### 🔘 **Buttons**
- Primary buttons: Gradient `#005461` → `#018790`
- Hover shadow: `#00B7B5` with opacity
- Success buttons: Keep green gradient
- Danger buttons: Keep red gradient

### 📊 **Statistics Cards**
- Background gradient: `#018790` → `#00B7B5`
- Text: White for contrast

### 📦 **Form Elements**
- Input focus border: `#00B7B5` (Light Teal)
- Section headers: `#F4F4F4` background with `#005461` text

### 📋 **Logs Panel**
- Container border: `#005461` (Dark Teal)
- Background: Dark gray `#2a2a2a` for readability
- Text: Light gray `#d4d4d4`

### ⏳ **Loading Spinner**
- Border: `#F4F4F4` (Light Gray)
- Active segment: `#00B7B5` (Light Teal)

## Layout

### Full Width Design
- Container: `width: 100%` (no max-width restriction)
- Sections span the entire viewport
- Responsive padding: `0 20px` on sides
- Grid layout maintains 2-column structure on desktop

## Responsive Breakpoints

```css
@media (max-width: 1024px) {
    .main-grid {
        grid-template-columns: 1fr;  /* Single column on tablets/mobile */
    }
}
```

## Color Accessibility

✅ **High Contrast**: Dark teal (#005461) on white provides excellent readability  
✅ **Visual Hierarchy**: Three shades of teal create clear differentiation  
✅ **Accent Color**: Light teal (#00B7B5) stands out for interactive elements  
✅ **Neutral Background**: Light gray (#F4F4F4) provides visual rest  

## Before & After

### Before (Purple Theme)
- Primary: `#667eea` (Purple blue)
- Secondary: `#764ba2` (Dark purple)
- Max width: 1400px (centered with margins)

### After (Teal Theme)
- Primary: `#005461` (Dark teal)
- Medium: `#018790` (Medium teal)
- Accent: `#00B7B5` (Light teal)
- Background: `#F4F4F4` (Light gray)
- Full width: 100% (spans entire viewport)

## CSS Variables (Future Enhancement)

Consider adding CSS variables for easier theme customization:

```css
:root {
    --color-primary-dark: #005461;
    --color-primary-medium: #018790;
    --color-accent: #00B7B5;
    --color-background-light: #F4F4F4;
    --color-text-dark: #333;
    --color-text-light: #666;
}
```

## Preview

🌐 **View the new design:**
```
http://localhost:5009
```

The Web UI now features:
- Professional teal color scheme
- Full-width modern layout
- Consistent color application across all elements
- Enhanced visual hierarchy
- Better brand alignment

---

**Applied:** ✅  
**Container Status:** Running  
**Port:** 5009  
**Persistence:** Enabled (./mysql_psql_data/)

