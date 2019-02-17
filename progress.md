First checkbox shows implementation, curly braces indicate test written.

Interface:
- [ ] fuzzy bar
	- [x] with simple index
	- [x] with advanced index
	- [x] async
	- [x] simple find
	- [ ] regex find
	- [ ] highlight opened
	- [ ] filter opened
	- [ ] show keyboard shortcut
	- [ ] force keyboard shortcut (learning mode)
- [ ] all-commands bar
	- [ ] display
	- [ ] act
	- [ ] keyboard shortcuts
- [ ] loading files
	- [x] on startup
	- [x] via fuzzy file bar (ctrl-o)
	- [x] via open file dialog
	- [ ] big files and load errors
- [ ] saving files
	- [x] in-place
	- [ ] via save-as dialog
	- [ ] autosaving
- [ ] edition
	- [x] cursor navigation (written, but with small bug)
		- [x] arrows
		- [x] pg-up pg-down
	- [x] multicursors (as above)
	- [ ] clipboard
		- [x] paste
		- [ ] copy
		- [ ] multiline
	- [ ] selection
		- [ ] single
		- [ ] multi
		- [ ] ctrl-a
	- [x] typing
	- [ ] undo
		- [x] basic
		- [ ] smart (merge insignifficant changes into bigger ones)
	- [ ] redo
- [ ] changing buffers
	- [x] via bufferlist
	- [ ] via keyboard next/prev
- [ ] bookmarking
	- [ ] anonymous
	- [ ] named
	- [ ] jumping
	- [ ] updating position on inside changes
	- [ ] updating position on outside changes
- [ ] code navigation
	- [ ] jump to symbol
	- [ ] jump back
	- [ ] cursor history
- [ ] colors
	- [x] syntax highlighting
	- [ ] select theme

- [ ] tinder-like browsing of "similar pages"
	- [ ] basic
	- [ ] with mid-processing
- [ ] plugin support
	- [ ] references fetch
	- [ ] references mid-processing
	- [ ] symbol navigation

- [ ] language server protocol
	- [ ] autocompletion
	- [ ] building references graph
	- [ ] multiple sources of data

after MVP:

- [ ] localized history