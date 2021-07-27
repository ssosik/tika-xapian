ZLIBVER = 1.2.11
ZLIB = zlib-$(ZLIBVER)
ZLIBZ = $(ZLIB).tar.gz
XPCOREVER = 1.2.25
XPCORE = xapian-core-$(XPCOREVER)
XPCOREZ = $(XPCORE).tar.xz

build: $(ZLIB) $(XPCORE)
	cargo build

# Fetch dependencies
$(ZLIBZ):
	wget https://zlib.net/$(ZLIBZ)

$(XPCOREZ):
	wget https://oligarchy.co.uk/xapian/$(XPCOREVER)/$(XPCOREZ)

$(ZLIB): $(ZLIBZ)
	tar -xvzf $(ZLIBZ)
	cd $(ZLIB) && ./configure && $(MAKE)

$(XPCORE): $(XPCOREZ)
	tar -xvf $(XPCOREZ)
	cp xapian-rusty/xapian-patch/api/omenquire.cc $(XPCORE)/api/omenquire.cc
	cp xapian-rusty/xapian-patch/include/xapian/enquire.h $(XPCORE)/include/xapian/enquire.h
	cd $(XPCORE) \
		&& ./configure CPPFLAGS=-I../$(ZLIB) LDFLAGS=-L../$(ZLIB) \
		&& $(MAKE)

clean:
	rm -rf $(ZLIB) $(XPCORE)
	cargo clean
