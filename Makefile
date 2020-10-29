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
	sudo install -d "$(DESTDIR)$(PREFIX)/bin"
	sudo install -t "$(DESTDIR)$(PREFIX)/bin" ./target/release/danivim
	sudo install -d "$(DESTDIR)$(PREFIX)/share/danivim"
	sudo cp -r ./runtime "$(DESTDIR)$(PREFIX)/share/danivim"
	sudo install -d "$(DESTDIR)$(PREFIX)/share/applications"
	sudo sh -c 'sed -e "s|Exec=danivim|Exec=$(PREFIX)/bin/danivim|" "./desktop/danivim.desktop" > "$(DESTDIR)$(PREFIX)/share/applications/danivim.desktop"'
	
	sudo install -d "$(DESTDIR)$(PREFIX)/share/icons/hicolor/48x48/apps"
	sudo install -d "$(DESTDIR)$(PREFIX)/share/icons/hicolor/128x128/apps"
	sudo cp ./desktop/danivim_128.png "$(DESTDIR)$(PREFIX)/share/icons/hicolor/128x128/apps/gnvim.png"
	sudo cp ./desktop/danivim_48.png "$(DESTDIR)$(PREFIX)/share/icons/hicolor/48x48/apps/gnvim.png"
	echo -e "backing up old nvimrc to ~/.nvimrcold"
	cp ~/.config/nvim/init.vim ~/.nvimrcold
	cp ./desktop/init.vim ~/.config/nvim/init.vim

uninstall:
	sudo rm "$(DESTDIR)$(PREFIX)/bin/danivim"
	sudo rm -rf "$(DESTDIR)$(PREFIX)/share/danivim"
	echo -e "uninstalling nvim config, replacing with old one"
	cp ~/.nvimrcold ~/.config/nvim/init.vim
