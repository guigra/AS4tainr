**FREE
// Stage 2, S2.91: data imports and exports. TOTAL is imported from the
// module GN91MOD, LABEL exported to it (INZ 'x'); each side sees the
// other's changes across the calls. Captured on PUB400
// (tools/golden/native91.txt n91_data) for the native unit NDATAIMP.
dcl-s total packed(7:0) import;
dcl-s label char(10) export inz('x');
dcl-pr addIt;
  n packed(7:0) const;
end-pr;
dcl-s line varchar(60);
line = %char(total) + ' ' + %trim(label);
addIt(5);
line += ' ' + %char(total) + ' ' + %trim(label);
total = 1;
label = 'y';
addIt(1);
line += ' ' + %char(total) + ' ' + %trim(label);
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
