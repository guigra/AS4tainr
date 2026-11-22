**FREE
// Interpreter fix (fork branch fix/split-array-rest): an array of five
// elements assigned a list of fewer values, %SPLIT, %LIST and a smaller
// array: whether the elements after them keep their values. Captured on
// PUB400 (tools/golden/native212.txt n212_listassign).
ctl-opt dftactgrp(*no);
dcl-s w varchar(10) dim(5);
dcl-s s varchar(10) dim(3);
dcl-s line varchar(200);

w(4) = 'four';
w(5) = 'five';
w = %split('a b');
line = w(1) + w(2) + '[' + w(3) + '][' + w(4) + '][' + w(5) + ']|';
w(4) = 'four';
w(5) = 'five';
w = %list('c' : 'd');
line += w(1) + w(2) + '[' + w(3) + '][' + w(4) + '][' + w(5) + ']|';
w(4) = 'four';
w(5) = 'five';
s(1) = 'e';
s(2) = 'f';
s(3) = 'g';
w = s;
line += w(1) + w(2) + w(3) + '[' + w(4) + '][' + w(5) + ']';
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
