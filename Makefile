ZLIBVER = 1.2.11
ZLIB = zlib-$(ZLIBVER)
ZLIBZ = $(ZLIB).tar.gz
XPCOREVER = 1.4.17
XPCORE = xapian-core-$(XPCOREVER)
XPCOREZ = $(XPCORE).tar.xz

build: $(ZLIB) $(XPCORE)/.libs
	cargo build

# Fetch dependencies
$(ZLIBZ):
	wget https://zlib.net/$(ZLIBZ)

$(XPCOREZ):
	wget https://oligarchy.co.uk/xapian/$(XPCOREVER)/$(XPCOREZ)

$(ZLIB): $(ZLIBZ)
	tar -xvzf $(ZLIBZ)
	cd $(ZLIB) \
		&& ./configure --static \
		&& $(MAKE)

$(XPCORE)/.libs: $(XPCOREZ) $(ZLIB)
	tar -xvf $(XPCOREZ)
	cd $(XPCORE) \
		&& ./configure CPPFLAGS=-I../$(ZLIB) LDFLAGS=-L../$(ZLIB) \
		&& $(MAKE)
		#&& ./configure CPPFLAGS=-I../$(ZLIB) LDFLAGS=-L../$(ZLIB) \
		#&& ./configure CXX='clang' CXXFLAGS='-arch=x86_64' CPPFLAGS=-I../$(ZLIB) LDFLAGS=-L../$(ZLIB) \
		#&& ./configure CXX='clang++' CPPFLAGS=-I../$(ZLIB) LDFLAGS=-L../$(ZLIB) \

$(XPCORE): $(XPCORE)/.libs

clean:
	rm -rf $(ZLIB) $(XPCORE)
	cargo clean
