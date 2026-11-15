**FREE
// Stage 2, DIM(*AUTO) and DIM(*VAR) arrays: growth on assignment, *NEXT,
// an index past the elements in use, %ELEM read and set (new elements'
// initial values), and reading past the elements in use. Indexes and
// counts past the dimension are variables: IBM i rejects constant ones at
// compile time (RNF0530, RNF7563; first version of this source). Captured on
// PUB400 (tools/golden/native47.txt n47_var) for the native unit NVARDIM.
ctl-opt dftactgrp(*no);
dcl-s au int(10) dim(*auto:5);
dcl-s va char(3) dim(*var:4);
dcl-s n int(10);
dcl-s t varchar(60);
dcl-s k int(10) inz(3);
dcl-s k6 int(10) inz(6);
dcl-s n5 int(10) inz(5);
say('E0 ' + %char(%elem(au)) + ' ' + %char(%elem(va)));
au(2) = 7;
say('AUTO2 ' + %char(%elem(au)) + ' ' + %char(au(1)) + ' ' + %char(au(2)));
au(*next) = 9;
say('NEXT ' + %char(%elem(au)) + ' ' + %char(au(3)));
monitor;
  au(k6) = 1;
  say('AUTO6 ' + %char(%elem(au)));
on-error;
  say('AUTO6 ERR ' + %char(%status));
endmon;
monitor;
  va(1) = 'abc';
  say('VAR1 ' + %char(%elem(va)));
on-error;
  say('VAR1 ERR ' + %char(%status));
endmon;
%elem(va) = 2;
va(2) = 'xyz';
t = '[' + va(1) + '][' + va(2) + ']';
say('VARSET ' + %char(%elem(va)) + ' ' + t);
monitor;
  va(k) = 'qqq';
  say('VAR3 ' + %char(%elem(va)));
on-error;
  say('VAR3 ERR ' + %char(%status));
endmon;
monitor;
  n = au(4);
  say('READ4 ' + %char(n));
on-error;
  say('READ4 ERR ' + %char(%status));
endmon;
%elem(au) = 1;
say('SHRINK ' + %char(%elem(au)) + ' ' + %char(au(1)));
%elem(au) = 3;
say('REGROW ' + %char(%elem(au)) + ' ' + %char(au(2)) + ' ' + %char(au(3)));
%elem(va) = 4;
t = '[' + va(3) + '][' + va(4) + ']';
say('VARGROW ' + %char(%elem(va)) + ' ' + t);
monitor;
  %elem(va) = n5;
  say('VARSET5 ' + %char(%elem(va)));
on-error;
  say('VARSET5 ERR ' + %char(%status));
endmon;
*inlr = *on;
// Write one line to gn1.out in the job's current directory (DSPLY under
// `system` goes to QSYSOPR, a queue PUB400 users share).
dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s line varchar(200);
  line = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :line,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
