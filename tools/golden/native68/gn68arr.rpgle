**FREE
// Stage 2, S2.68: %XFOOT, %MAXARR and %MINARR of whole arrays. A packed
// array with decimals and negative values, ties (the first one counts),
// an integer array, a character array. Captured on PUB400
// (tools/golden/native68.txt n68_arr) for the native unit NARRFN.
ctl-opt dftactgrp(*no);
dcl-s p packed(7 : 2) dim(5);
dcl-s q int(10) dim(4);
dcl-s c char(3) dim(4);
dcl-s t packed(11 : 2);
p(1) = 10.50;
p(2) = -3.25;
p(3) = 99.99;
p(4) = 99.99;
p(5) = -3.25;
q(1) = 7;
q(2) = 7;
q(3) = -2;
q(4) = 5;
c(1) = 'bob';
c(2) = 'amy';
c(3) = 'zed';
c(4) = 'amy';
t = %xfoot(p);
say('XFOOT P ' + %char(t));
say('XFOOT Q ' + %char(%xfoot(q)));
say('MAX P ' + %char(%maxarr(p)) + ' MIN P ' + %char(%minarr(p)));
say('MAX Q ' + %char(%maxarr(q)) + ' MIN Q ' + %char(%minarr(q)));
say('MAX C ' + %char(%maxarr(c)) + ' MIN C ' + %char(%minarr(c)));
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s line varchar(200);
  line = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :line,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
