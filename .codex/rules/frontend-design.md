# Frontend Design Rule

Use this rule when changing user-visible UI, CSS, layout, component composition, route presentation, loading states, toasts, empty/error states, or visual assets in this Dioxus Bitcoin Lightning Game workspace.

## Direction

- Keep the template quiet, readable, and easy to repopulate.
- Favor reusable app structure over product-specific decoration.
- Preserve the top bar, theme toggle, language dropdown, toast region, and current four-page route shape unless a future spec changes them.
- Keep copy and imagery generic; `Documentation/Images/Screenshot01.png` and `Documentation/Images/Infographic01.png` are replaceable slots.

## Layout

- Keep `Home`, `Set Up`, `Play Game`, and `Debug Network` visually consistent.
- Use dense but comfortable spacing and stable dimensions for top navigation controls.
- Avoid nested cards and avoid decorative orbs/blobs.
- Make text fit in controls at desktop and mobile widths; collapse nav labels to short labels before allowing wrapping.

## Verification

- For browser-visible changes, run the real web app and inspect the result when practical.
- If the web server is already running on the target port, stop it and restart it before trusting the browser result.
