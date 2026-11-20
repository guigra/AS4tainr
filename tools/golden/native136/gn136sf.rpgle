**FREE
// Stage 2, S2.136: figurative constants assigned to %SUBST: *BLANKS,
// *ALL'-', *ZEROS to the end of the field, and *HIVAL. Captured on
// PUB400 (tools/golden/native136.txt n136_substfig) for the native unit
// NSUBFIG.
ctl-opt dftactgrp(*no);
dcl-s x char(8) inz('abcdefgh');
dcl-s line varchar(80);

%subst(x: 2: 3) = *blanks;
line = '[' + x + ']';
%subst(x: 1: 2) = *all'-';
line += '[' + x + ']';
%subst(x: 5) = *zeros;
line += '[' + x + ']';
%subst(x: 3: 1) = *hival;
if %subst(x: 3: 1) = x'FF';
  line += 'h';
endif;
say(line);
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
