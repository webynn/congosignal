// Local PDF OCR using macOS Vision; original pages are never modified.
import Foundation
import PDFKit
import Vision
import AppKit
let args=CommandLine.arguments
let doc=PDFDocument(url:URL(fileURLWithPath:args[1]))!
var pages:[[String:Any]]=[]
for i in 0..<doc.pageCount {
 let page=doc.page(at:i)!, bounds=doc.page(at:i)!.bounds(for:.mediaBox)
 let image=page.thumbnail(of:NSSize(width:bounds.width*2,height:bounds.height*2),for:.mediaBox)
 var rect=CGRect(origin:.zero,size:image.size)
 guard let cg=image.cgImage(forProposedRect:&rect,context:nil,hints:nil) else { fatalError("render failed") }
 let request=VNRecognizeTextRequest();request.recognitionLevel = .accurate
 request.recognitionLanguages=["fr-FR","en-US"];request.usesLanguageCorrection=true
 try VNImageRequestHandler(cgImage:cg).perform([request])
 let text=(request.results ?? []).compactMap{$0.topCandidates(1).first?.string}.joined(separator:"\n")
 pages.append(["page":i+1,"text":text]);fputs("page \(i+1)/\(doc.pageCount)\n",stderr)
}
let data=try JSONSerialization.data(withJSONObject:pages,options:[.sortedKeys])
try data.write(to:URL(fileURLWithPath:args[2]))
