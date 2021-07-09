ZLIB = zlib-1.2.11
XPCORE = xapian-core-1.4.17

build: $(ZLIB) $(XPCORE)
	cargo build

$(ZLIB):
	tar -xvzf $(ZLIB).tar.gz
	cd $(ZLIB) && ./configure && $(MAKE)

$(XPCORE):
	tar -xvf $(XPCORE).tar.xz
	cd $(XPCORE) \
		&& ./configure CPPFLAGS=-I../$(ZLIB) LDFLAGS=-L../$(ZLIB) \
		&& $(MAKE)

clean:
	rm -rf $(ZLIB) $(XPCORE)
	cargo clean
