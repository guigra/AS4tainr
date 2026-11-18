      * Stage 2, S2.78: EXCEPT to a program-described printer file. Named
      * exception records (HDR with PAGE edited Z, DTL with a character
      * field, edit codes J and Z and a field conditioned by an
      * indicator), an unnamed one (EXCEPT with no name, space after 2),
      * TOT with a field cleared after it is printed (blank after) and an
      * edit word, then DTL again. The program reads its own spooled file
      * back (SYSTOOLS.SPOOLED_FILE_DATA). Captured on PUB400
      * (tools/golden/native78.txt n78_except) for the native unit NEXCPT.
       ctl-opt dftactgrp(*no);
     FQPRINT    O    F  132        PRINTER OFLIND(*INOF)
       dcl-s amt packed(7:2);
       dcl-s nm char(10);
       dcl-s qty zoned(5:0);
       dcl-s line varchar(132);
       exec sql set option commit = *none;
       amt = 1234.5;
       nm = 'ALPHA';
       qty = 7;
       *in50 = *on;
       except hdr;
       except dtl;
       amt = -42;
       nm = 'BETA';
       *in50 = *off;
       except dtl;
       except;
       except tot;
       except dtl;
       close qprint;
       exec sql declare c cursor for select rtrim(spooled_data)
         from table(systools.spooled_file_data(job_name => '*',
                                               spooled_file_name => 'QPRINT'))
         order by ordinal_position;
       exec sql open c;
       dow 1 = 1;
         exec sql fetch c into :line;
         if sqlcode <> 0;
           leave;
         endif;
         say('|' + line + '|');
       enddo;
       exec sql close c;
       *inlr = *on;
     OQPRINT    E            HDR            1
     O                                           10 'REPORT'
     O                       PAGE          Z     20
     OQPRINT    E            DTL            1
     O                       NM                  12
     O                       AMT           J     25
     O                       QTY           Z     32
     O               50                          40 'FLAG'
     OQPRINT    E                           2
     O                                           10 'UNNAMED'
     OQPRINT    E            TOT            1
     O                       QTY           ZB    10
     O                                           15 'TOT'
     O                       AMT                 30 '  ,  0.  -'
       dcl-proc say;
         dcl-pi *n;
           text varchar(200) const;
         end-pi;
         dcl-s wbuf varchar(200);
         wbuf = text;
         exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out',
                    line => :wbuf, overwrite => 'APPEND',
                    end_of_line => 'LF');
       end-proc;
