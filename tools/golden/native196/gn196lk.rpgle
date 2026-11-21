**FREE
// Stage 2, S2.196: %LOOKUP of a subfield of every element of an array of
// data structures, DSARR(*).SUBFIELD: a character and a numeric subfield,
// a start element, a value not found (%LOOKUPxx needs a sequenced array:
// RNF0592 for one of these). Captured on
// PUB400 (tools/golden/native196.txt n196_lookds) for the native unit
// NLOOKDS.
ctl-opt dftactgrp(*no);
dcl-ds item qualified dim(4);
  name char(10);
  qty int(10);
end-ds;
dcl-s line varchar(200);

item(1).name = 'APPLE';
item(1).qty = 5;
item(2).name = 'PEAR';
item(2).qty = 9;
item(3).name = 'PLUM';
item(3).qty = 12;
item(4).name = 'PEAR';
item(4).qty = 20;
line = %char(%lookup('PEAR' : item(*).name)) + ' '
     + %char(%lookup('KIWI' : item(*).name)) + ' '
     + %char(%lookup('PEAR' : item(*).name : 3)) + ' '
     + %char(%lookup(12 : item(*).qty));
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
