**FREE
// Stage 2, S2.179: an array assigned %SPLIT: runs of blanks, a separator
// argument, *ALLSEP, the elements after the pieces initialized, and a
// DIM(*AUTO) array's elements in use. Captured on PUB400
// (tools/golden/native179.txt n179_split) for the native unit NSPLITA.
ctl-opt dftactgrp(*no);
dcl-s w varchar(10) dim(5);
dcl-s c char(4) dim(4);
dcl-s v varchar(10) dim(*auto : 5);
dcl-s line varchar(120);

w(5) = 'old';
w = %split('the quick  brown fox');
line = w(1) + '/' + w(4) + '/' + w(5) + '|';
c = %split('a,,b,c' : ',');
line += c(1) + c(2) + c(3) + '[' + c(4) + ']|';
c = %split('a,,b' : ',' : *allsep);
line += c(1) + '[' + c(2) + ']' + c(3) + '|';
v = %split('x y');
line += %char(%elem(v)) + v(2);
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
