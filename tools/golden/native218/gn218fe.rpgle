**FREE
// Stage 2, S2.213: FOR-EACH over %SPLIT (a separator, *ALLSEP, the
// default blanks) and over a DIM(*AUTO) array with three of its five
// elements in use. Captured on PUB400 (tools/golden/native218.txt
// n218_foreach) for the native unit NEACHSP.
ctl-opt dftactgrp(*no);
dcl-s item varchar(10);
dcl-s auto varchar(10) dim(*auto : 5);
dcl-s line varchar(200);

for-each item in %split('a,b,,c' : ',');
  line += '[' + item + ']';
endfor;
line += '|';
for-each item in %split('a,b,,c' : ',' : *allsep);
  line += '[' + item + ']';
endfor;
line += '|';
for-each item in %split('  the quick  fox ');
  line += '[' + item + ']';
endfor;
line += '|';
auto(*next) = 'x';
auto(*next) = 'y';
auto(*next) = 'z';
for-each item in auto;
  line += '[' + item + ']';
endfor;
line += '|' + %char(%elem(auto));
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
