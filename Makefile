ifeq ($(PREFIX),)
    PREFIX := /usr/local
endif

ifdef NOWEBKIT2GTK
    FEATURES := --no-default-features
endif

build:
	cargo build --release $(FEATURES)

syntect-pack:
	git submodule update --init
	find sublime-syntaxes/sources \
	    -name '*.sublime-syntax' \
	    -exec cp -- "{}" sublime-syntaxes/syntaxes/ \;
	cargo run --example build-syntect-pack

install:
	install -d "$(DESTDIR)$(PREFIX)/bin"
	install -t "$(DESTDIR)$(PREFIX)/bin" ./target/release/danivim
	install -d "$(DESTDIR)$(PREFIX)/share/danivim"
	cp -r ./runtime "$(DESTDIR)$(PREFIX)/share/danivim"
	install -d "$(DESTDIR)$(PREFIX)/share/applications"
	sed -e "s|Exec=danivim|Exec=$(PREFIX)/bin/gnvim|" \
	    "./desktop/danivim.desktop" \
	    >"$(DESTDIR)$(PREFIX)/share/applications/danivim.desktop"
	install -d "$(DESTDIR)$(PREFIX)/share/icons/hicolor/48x48/apps"
	install -d "$(DESTDIR)$(PREFIX)/share/icons/hicolor/128x128/apps"
	cp ./desktop/danivim_128.png "$(DESTDIR)$(PREFIX)/share/icons/hicolor/128x128/apps/gnvim.png"
	cp ./desktop/danivim_48.png "$(DESTDIR)$(PREFIX)/share/icons/hicolor/48x48/apps/gnvim.png"

uninstall:
	rm "$(DESTDIR)$(PREFIX)/bin/danivim"
	rm -rf "$(DESTDIR)$(PREFIX)/share/danivim"
