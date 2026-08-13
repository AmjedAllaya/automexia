#!/usr/bin/env perl

# Automexia interactive eza presentation filter.
#
# eza versions before 0.19.2 cannot override icons by basename, and their
# EZA_COLORS glob rules do not override the directory kind.  Automexia still
# supports those commonly packaged versions (including Ubuntu's 0.18.x) by
# replacing only eza's known generic folder glyph immediately before a
# recognized directory basename.  Filenames and non-interactive output are
# never rewritten.

use strict;
use warnings;
use utf8;

my @folder_roles = (
    [ qr/^\.?(?:secret|secrets|private|credentials?|keys?|vault|certs?|certificates?)$/i, 0xF0250, '255;92;122' ],
    [ qr/^\.?(?:config|configs|configuration|settings|etc|profiles?)$/i,                 0xF107F, '255;176;32' ],
    [ qr/^\.?(?:logs?|logfiles?|traces?|telemetry)$/i,                                 0xF0C82, '242;201;76' ],
    [ qr/^(?:apps?|src|source|lib|libs|librio(?:-wasm)?|rio-.+|corcovado|sugarloaf|teletypewriter)$/i, 0xF19F6, '80;213;255' ],
    [ qr/^(?:docs?|documentation|guides?|wiki|examples?)$/i,                            0xF10B7, '96;211;148' ],
    [ qr/^(?:tests?|specs?|__tests__|fixtures?|mocks?|fuzz|benches?|benchmarks?)$/i,      0xF197E, '220;120;255' ],
    [ qr/^(?:target|build|dist|out|output|release|debug|artifacts?|coverage|changes)$/i,  0xF0D0B, '244;111;97' ],
    [ qr/^(?:assets?|public|static|media|images?|icons?|fonts?|resources?)$/i,            0xF024F, '244;114;182' ],
    [ qr/^\.?(?:cargo|node_modules|vendor|vendors|packages?|deps?|dependencies)$/i,      0xF0253, '167;139;250' ],
    [ qr/^\.?(?:git|github|gitlab)$/i,                                                   0xE5FB, '220;120;255' ],
    [ qr/^(?:bin|scripts?|tools?|shell-integration|ci)$/i,                               0xF19FC, '45;212;191' ],
    [ qr/^(?:data|db|database|storage|migrations?|schemas?)$/i,                          0xF12E3, '129;140;248' ],
    [ qr/^\.?(?:cache|tmp|temp|sessions?|backups?)$/i,                                  0xF0ABA, '148;163;184' ],
    [ qr/^(?:infra|infrastructure|terraform|k8s|kubernetes|helm|docker|cloud)$/i,          0xF0870, '36;150;237' ],
    [ qr/^(?:packaging|package)$/i,                                                       0xF06EB, '255;176;32' ],
);

my $csi = qr/\e\[[0-9;]*m/;
my $generic_folder = qr/[\x{E5FF}\x{F07B}\x{F024B}]/;

while (my $line = <STDIN>) {
    # eza emits one generic folder glyph, a space, optional ANSI styling, and
    # the literal basename.  Replacing the one-cell glyph with another
    # one-cell glyph preserves grid/long/tree alignment and copyable names.
    $line =~ s{
        (?:$csi)* $generic_folder [ ] (?:$csi)*
        ([^\e\r\n]+?)
        (?=(?:$csi)*(?:[/\\])?(?:[ ]{2,}|\r?$))
    }{
        my $original = $&;
        my $name = $1;
        my ($role) = grep { $name =~ $_->[0] } @folder_roles;
        if ($role) {
            "\e[38;2;$role->[2]m" . chr($role->[1]) . " \e[1m" . $name;
        } else {
            $original;
        }
    }gex;
    print $line;
}
