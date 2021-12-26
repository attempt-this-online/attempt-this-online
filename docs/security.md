# Security Policy
Attempt This Online is a new project and there are bound to be security vulnerabilities.

If you find a security vulnerability, email me at any email address ending in `@pxeger.com`. Do not report it as an
issue in this repository.

The setup script generates "flags", which are random text stored in locations that should only be readable by various
users. You can use the contents of these flags to demostrate your vulnerability. The flags are stored in:
- `/root/flag` (readable only by root)
- `/var/local/lib/ATO_home/flag` (readable only by the `ato` user)

This security policy does not give you permission to attempt to attack the official instance (ato.pxeger.com) or any of
my servers. Test and demonstrate vulnerabilities only on your own personal instance.

Note that the most effort will be put into supporting appliances using the canonical setup script; while I will do my
best to make this project secure in all circumstances, vulnerabilities relating exclusively to different setups may not
be prioritised.

This software is provided ENTIRELY WITHOUT WARRANTY and the author(s) take NO RESPONSIBILITY for any damage caused as a
result of its insecurity. See the [licence](./LICENCE.txt) for further details.
