#!/bin/sh
set -eu

THEMES_DIR=static/Themes

if [ ! -d "$THEMES_DIR" ]; then
  echo "Themes directory not found at $THEMES_DIR"
  exit 1
fi

for theme_dir in "$THEMES_DIR"/*; do
  if [ -d "$theme_dir" ]; then
    stylesheet_file="$theme_dir/stylesheet.json"
    if [ -f "$stylesheet_file" ]; then
      # Check if the file needs migration
      if ! grep -q '"ui":' "$stylesheet_file"; then
        echo "Migrating $stylesheet_file"
        
        # Backup the original file
        cp "$stylesheet_file" "$stylesheet_file.bak"
        
        # Create a temporary file for the new structure
        tmp_file=$(mktemp)

        # Use a here document to build the new file
        # This is a much cleaner and more robust approach
        (
          # Preserve wallpaper
          grep '"wallpaper"' "$stylesheet_file.bak"
          
          # UI section
          echo '  "ui": {'
          grep '"margin_x"' "$stylesheet_file.bak"
          grep '"margin_y"' "$stylesheet_file.bak"
          grep '"list_margin"' "$stylesheet_file.bak"
          grep '"padding_x"' "$stylesheet_file.bak"
          grep '"padding_y"' "$stylesheet_file.bak"
          sed -n '/"ui_font": {/,/}/p' "$stylesheet_file.bak" | sed 's/"path":/"path": "/;s/ttf/ttf"/' | sed '/"size":/s/$/,/'
          grep '"foreground_color"' "$stylesheet_file.bak" | sed 's/foreground_color/text_color/'
          grep '"stroke_color"' "$stylesheet_file.bak" | sed 's/stroke_color/text_stroke_color/'
          grep '"background_color"' "$stylesheet_file.bak"
          grep '"highlight_color"' "$stylesheet_file.bak"
          grep '"highlight_text_color"' "$stylesheet_file.bak"
          grep '"highlight_text_stroke_color"' "$stylesheet_file.bak"
          grep '"disabled_color"' "$stylesheet_file.bak"
          grep '"tab_font_size"' "$stylesheet_file.bak"
          grep '"tab_color"' "$stylesheet_file.bak"
          grep '"tab_stroke_color"' "$stylesheet_file.bak"
          grep '"tab_selected_color"' "$stylesheet_file.bak"
          grep '"tab_selected_stroke_color"' "$stylesheet_file.bak"
          grep '"stroke_width"' "$stylesheet_file.bak" | sed 's/,$//'
          echo '  },'

          # Status bar section
          echo '  "status_bar": {'
          grep '"show_battery_level"' "$stylesheet_file.bak"
          grep '"show_clock"' "$stylesheet_file.bak"
          grep '"status_bar_font_size"' "$stylesheet_file.bak" | sed 's/status_bar_font_size/font_size/'
          grep '"status_bar_color"' "$stylesheet_file.bak" | sed 's/status_bar_color/text_color/'
          grep '"status_bar_stroke_color"' "$stylesheet_file.bak" | sed 's/status_bar_stroke_color/text_stroke_color/' | sed 's/,$//'
          echo '  },'

          # Button hints section
          echo '  "button_hints": {'
          grep '"button_hint_font_size"' "$stylesheet_file.bak"
          grep '"button_a_color"' "$stylesheet_file.bak"
          grep '"button_b_color"' "$stylesheet_file.bak"
          grep '"button_x_color"' "$stylesheet_file.bak"
          grep '"button_y_color"' "$stylesheet_file.bak"
          grep '"button_bg_color"' "$stylesheet_file.bak"
          grep '"button_text_color"' "$stylesheet_file.bak"
          grep '"button_hint_text_color"' "$stylesheet_file.bak" | sed 's/button_hint_text_color/text_color/' | sed 's/,$//'
          echo '  },'

          # Recents section
          echo '  "recents": {'
          grep '"use_recents_carousel"' "$stylesheet_file.bak" | sed 's/,$//'
          echo '  },'

          # Games section
          echo '  "games": {'
          grep '"boxart_width"' "$stylesheet_file.bak" | sed 's/,$//'
          echo '  },'

          # Menu section
          echo '  "menu": {'
          grep '"menu_background_color"' "$stylesheet_file.bak" | sed 's/menu_background_color/background_color/'
          sed -n '/"guide_font": {/,/}/p' "$stylesheet_file.bak" | sed 's/"path":/"path": "/;s/ttf/ttf"/' | sed '/"size":/s/$/,/' | sed 's/,$//'
          echo '  }'
        ) | sed '1s/^/{\n/' | sed '$a\n}' > "$tmp_file"
        
        # Replace the old file
        mv "$tmp_file" "$stylesheet_file"
        echo "Finished migrating $stylesheet_file"
      fi
    fi
  fi
done

echo "Theme migration script finished."