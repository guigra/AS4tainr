      * Stage 2, S2.154: MOVEA with a literal index, into an element of
      * the result array and from an element of factor 2's, with and
      * without P. Captured on PUB400 (tools/golden/native154.txt
      * n154_moveal) for the native unit NMOVEAL.
     H DFTACTGRP(*NO)
     D CH              S              1    DIM(6)
     D A3              S              2    DIM(3) INZ('..')
     D B3              S              2    DIM(3) INZ('..')
     D LINE            S             40    VARYING
     D K               S             10I 0
     C                   MOVEA     'ABCDEF'      CH
     C                   MOVEA     'XY'          CH(2)
     C                   MOVEA     CH(4)         A3
     C                   MOVEA(P)  CH(5)         B3(2)
       for k = 1 to 6;
         line += ch(k);
       endfor;
       line += ' ' + a3(1) + a3(2) + a3(3) + ' ' + b3(1) + b3(2) + b3(3);
       say(line);
       *inlr = *on;

       dcl-proc say;
         dcl-pi *n;
           text varchar(200) const;
         end-pi;
         dcl-s wbuf varchar(200);
         wbuf = text;
         exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out',
                  line => :wbuf, overwrite => 'APPEND', end_of_line => 'LF');
       end-proc;
