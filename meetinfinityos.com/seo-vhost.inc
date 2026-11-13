# Included inside the InfinityOS HTTPS virtual host only.
RewriteEngine On
RewriteCond %{THE_REQUEST} "\s/+index\.html(?:[?\s])" [NC]
RewriteRule ^/?index\.html$ https://www.meetinfinityos.com/ [R=301,L]
RewriteCond %{HTTP_HOST} ^meetinfinityos\.com(?::443)?$ [NC]
RewriteRule ^ https://www.meetinfinityos.com%{REQUEST_URI} [R=301,L,NE]
